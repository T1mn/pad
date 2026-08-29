use super::ServerInput;
use super::{
    constant_time_eq, digest_secret, now, persist_disk_state, random_id, random_secret, ClientSink,
    ConnectionCloseReason, GatewayState, QueuedFrame, StoredDevice, CLIENT_LIMIT,
    CLIENT_QUEUE_FRAMES, MAX_REMOTE_FRAME_BYTES, PAIRING_MAX_ATTEMPTS, REMOTE_SUBPROTOCOL,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::Semaphore;
use tokio::time::{interval, timeout, MissedTickBehavior};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

pub(super) use super::tls::load_or_create_tls;

pub(super) fn spawn(
    listener: TcpListener,
    tls: Arc<ServerConfig>,
    shared: Arc<Mutex<GatewayState>>,
    owner_tx: SyncSender<ServerInput>,
    shutdown: Arc<AtomicBool>,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("pad-remote-wss".to_string())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(_) => return,
            };
            runtime.block_on(run(listener, tls, shared, owner_tx, shutdown));
        })
}

async fn run(
    listener: TcpListener,
    tls: Arc<ServerConfig>,
    shared: Arc<Mutex<GatewayState>>,
    owner_tx: SyncSender<ServerInput>,
    shutdown: Arc<AtomicBool>,
) {
    let listener = match TokioTcpListener::from_std(listener) {
        Ok(listener) => listener,
        Err(_) => return,
    };
    let acceptor = TlsAcceptor::from(tls);
    let slots = Arc::new(Semaphore::new(CLIENT_LIMIT));
    while !shutdown.load(Ordering::Acquire) {
        let accepted = match timeout(Duration::from_millis(200), listener.accept()).await {
            Ok(Ok(accepted)) => accepted,
            Ok(Err(_)) => continue,
            Err(_) => continue,
        };
        let _ = accepted.0.set_nodelay(true);
        let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else {
            continue;
        };
        let acceptor = acceptor.clone();
        let shared = Arc::clone(&shared);
        let owner_tx = owner_tx.clone();
        tokio::spawn(async move {
            let _slot = slot;
            let tls_stream =
                match timeout(Duration::from_secs(8), acceptor.accept(accepted.0)).await {
                    Ok(Ok(stream)) => stream,
                    _ => return,
                };
            let mut websocket_config = WebSocketConfig::default();
            websocket_config.max_message_size = Some(MAX_REMOTE_FRAME_BYTES);
            websocket_config.max_frame_size = Some(MAX_REMOTE_FRAME_BYTES);
            websocket_config.write_buffer_size = 0;
            let websocket = timeout(
                Duration::from_secs(8),
                tokio_tungstenite::accept_hdr_async_with_config(
                    tls_stream,
                    accept_pad_subprotocol,
                    Some(websocket_config),
                ),
            )
            .await;
            if let Ok(Ok(websocket)) = websocket {
                serve(websocket, shared, owner_tx).await;
            }
        });
    }
}

#[allow(
    clippy::result_large_err,
    reason = "tungstenite fixes the HTTP upgrade callback error type"
)]
fn accept_pad_subprotocol(
    request: &Request,
    mut response: Response,
) -> Result<Response, ErrorResponse> {
    let requested = request
        .headers()
        .get("sec-websocket-protocol")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|item| item.trim() == REMOTE_SUBPROTOCOL)
        });
    if !requested {
        let mut denied = ErrorResponse::new(Some("missing PAD subprotocol".to_string()));
        *denied.status_mut() = StatusCode::BAD_REQUEST;
        return Err(denied);
    }
    response.headers_mut().insert(
        "sec-websocket-protocol",
        HeaderValue::from_static(REMOTE_SUBPROTOCOL),
    );
    Ok(response)
}

async fn serve<S>(
    websocket: tokio_tungstenite::WebSocketStream<S>,
    shared: Arc<Mutex<GatewayState>>,
    owner_tx: SyncSender<ServerInput>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (mut sink, mut stream) = websocket.split();
    let (event_tx, mut event_rx) = tokio_mpsc::channel::<QueuedFrame>(CLIENT_QUEUE_FRAMES);
    let queued_bytes = Arc::new(AtomicUsize::new(0));
    let overflowed = Arc::new(AtomicBool::new(false));
    let (overflow_tx, mut overflow_rx) = tokio::sync::watch::channel(false);
    let (close_tx, mut close_rx) = tokio::sync::watch::channel(None);
    let acknowledged = Arc::new(AtomicU64::new(0));
    let mut authenticated: Option<String> = None;
    let mut connection_id: Option<String> = None;
    let mut heartbeat = interval(Duration::from_secs(10));
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let authentication_deadline = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(authentication_deadline);
    let mut last_seen = tokio::time::Instant::now();
    loop {
        tokio::select! {
            biased;
            _ = &mut authentication_deadline, if authenticated.is_none() => {
                let _ = send_error(&mut sink, "authentication_timeout", "pair or resume within 10 seconds").await;
                break;
            }
            changed = close_rx.changed(), if authenticated.is_some() => {
                let reason = close_rx.borrow_and_update().to_owned();
                if changed.is_err() || reason.is_some() {
                    let (code, message) = match reason {
                        Some(ConnectionCloseReason::RemoteDisabled) => ("remote_disabled", "remote access was disabled"),
                        Some(ConnectionCloseReason::ReplacedByNewConnection) => ("replaced_by_new_connection", "a newer connection replaced this one"),
                        _ => ("device_revoked", "remote device was revoked"),
                    };
                    let _ = send_error(&mut sink, code, message).await;
                    break;
                }
            }
            changed = overflow_rx.changed(), if authenticated.is_some() => {
                if changed.is_err() || *overflow_rx.borrow() {
                    let (server_epoch, latest_revision) = {
                        let state = shared.lock().unwrap_or_else(|error| error.into_inner());
                        (state.broker.epoch.clone(), state.broker.revision)
                    };
                    let _ = send_with_timeout(&mut sink, Message::Text(json!({
                        "type":"resync_required",
                        "reason":"slow_client",
                        "server_epoch":server_epoch,
                        "latest_revision":latest_revision,
                    }).to_string().into())).await;
                    break;
                }
            }
            _ = heartbeat.tick() => {
                if last_seen.elapsed() > Duration::from_secs(25) {
                    break;
                }
                if send_with_timeout(&mut sink, Message::Text(json!({"type":"ping","sent_at":now()}).to_string().into())).await.is_err() {
                    break;
                }
            }
            queued = event_rx.recv(), if authenticated.is_some() => {
                match queued {
                    Some(frame) => {
                        if close_rx.borrow().is_some() {
                            break;
                        }
                        let sent = send_with_timeout(&mut sink, Message::Text(frame.value.to_string().into())).await;
                        queued_bytes.fetch_sub(frame.bytes, Ordering::AcqRel);
                        if sent.is_err() {
                            break;
                        }
                    }
                    None if overflowed.load(Ordering::Acquire) => {
                        let (server_epoch, latest_revision) = {
                            let state = shared.lock().unwrap_or_else(|error| error.into_inner());
                            (state.broker.epoch.clone(), state.broker.revision)
                        };
                        let _ = send_with_timeout(&mut sink, Message::Text(json!({
                            "type":"resync_required",
                            "reason":"slow_client",
                            "server_epoch":server_epoch,
                            "latest_revision":latest_revision,
                        }).to_string().into())).await;
                        break;
                    }
                    None => break,
                }
            }
            incoming = stream.next() => {
                let Some(Ok(message)) = incoming else { break; };
                last_seen = tokio::time::Instant::now();
                match message {
                    Message::Text(text) if text.len() <= MAX_REMOTE_FRAME_BYTES => {
                        let Ok(frame) = serde_json::from_str::<IncomingFrame>(&text) else {
                            if send_error(&mut sink, "invalid_json", "invalid remote JSON frame").await.is_err() { break; }
                            continue;
                        };
                        match frame {
                            IncomingFrame::Pair { pairing_id, secret, device } => {
                                if authenticated.is_some() {
                                    if send_error(&mut sink, "already_authenticated", "connection is already authenticated").await.is_err() { break; }
                                    continue;
                                }
                                match pair(&shared, &pairing_id, &secret, device) {
                                    Ok((device_id, token, epoch, latest_revision)) => {
                                        let registered = match register_client(&shared, &device_id, latest_revision, event_tx.clone(), Arc::clone(&queued_bytes), Arc::clone(&overflowed), overflow_tx.clone(), close_tx.clone(), Arc::clone(&acknowledged)) {
                                            Ok(registered) => registered,
                                            Err((code, message)) => {
                                                rollback_paired_device(&shared, &device_id);
                                                let _ = send_error(&mut sink, code, message).await;
                                                break;
                                            }
                                        };
                                        authenticated = Some(device_id.clone());
                                        connection_id = Some(registered);
                                        let _ = owner_tx.send(ServerInput::RemoteStatusChanged);
                                        let frame = json!({"type":"paired","device_id":device_id,"device_token":token,"server_epoch":epoch,"latest_revision":latest_revision});
                                        if send_with_timeout(&mut sink, Message::Text(frame.to_string().into())).await.is_err() {
                                            rollback_paired_device(&shared, &device_id);
                                            break;
                                        }
                                    }
                                    Err((code, message)) => if send_error(&mut sink, code, message).await.is_err() { break; },
                                }
                            }
                            IncomingFrame::Resume { device_id, device_token, server_epoch, after_revision } => {
                                if authenticated.is_some() {
                                    if send_error(&mut sink, "already_authenticated", "connection is already authenticated").await.is_err() { break; }
                                    continue;
                                }
                                match resume(&shared, &device_id, &device_token, server_epoch.as_deref(), after_revision) {
                                    Ok(resume) => {
                                        let registered = match register_client(&shared, &device_id, resume.latest_revision, event_tx.clone(), Arc::clone(&queued_bytes), Arc::clone(&overflowed), overflow_tx.clone(), close_tx.clone(), Arc::clone(&acknowledged)) {
                                            Ok(registered) => registered,
                                            Err((code, message)) => {
                                                let _ = send_error(&mut sink, code, message).await;
                                                break;
                                            }
                                        };
                                        authenticated = Some(device_id.clone());
                                        connection_id = Some(registered);
                                        let _ = owner_tx.send(ServerInput::RemoteStatusChanged);
                                        if send_with_timeout(&mut sink, Message::Text(resume.welcome.to_string().into())).await.is_err() { break; }
                                        for frame in resume.replay {
                                            if send_with_timeout(&mut sink, Message::Text(frame.to_string().into())).await.is_err() { return finish(&shared, connection_id.as_deref(), &owner_tx); }
                                        }
                                    }
                                    Err((code, message)) => if send_error(&mut sink, code, message).await.is_err() { break; },
                                }
                            }
                            IncomingFrame::Command { command_id, action, params } => {
                                let Some(device_id) = authenticated.as_deref() else {
                                    if send_error(&mut sink, "not_authenticated", "pair or resume first").await.is_err() { break; }
                                    continue;
                                };
                                let outcome = execute_command(&shared, &owner_tx, device_id, &command_id, &action, params).await;
                                let mut frame = json!({"type":"command_result","command_id":command_id,"ok":outcome.ok,"result":outcome.result,"error":outcome.error});
                                if frame.to_string().len() > MAX_REMOTE_FRAME_BYTES {
                                    frame = json!({
                                        "type":"command_result",
                                        "command_id":command_id,
                                        "ok":false,
                                        "error":{"code":"response_too_large","message":"remote response exceeds 1 MiB; request a fresh snapshot"},
                                    });
                                }
                                if send_with_timeout(&mut sink, Message::Text(frame.to_string().into())).await.is_err() { break; }
                            }
                            IncomingFrame::Ack { through_revision } => {
                                acknowledged.fetch_max(through_revision, Ordering::AcqRel);
                            }
                            IncomingFrame::Ping => {
                                if send_with_timeout(&mut sink, Message::Text(json!({"type":"pong","sent_at":now()}).to_string().into())).await.is_err() { break; }
                            }
                            IncomingFrame::Pong => {}
                        }
                    }
                    Message::Text(_) => {
                        let _ = send_error(&mut sink, "frame_too_large", "remote frame exceeds 1 MiB").await;
                        break;
                    }
                    Message::Ping(data) => { if send_with_timeout(&mut sink, Message::Pong(data)).await.is_err() { break; } }
                    Message::Pong(_) => {}
                    Message::Close(_) => break,
                    _ => {
                        let _ = send_error(&mut sink, "invalid_frame", "remote protocol accepts JSON text frames only").await;
                        break;
                    }
                }
            }
        }
    }
    finish(&shared, connection_id.as_deref(), &owner_tx);
}

pub(super) fn finish(
    shared: &Arc<Mutex<GatewayState>>,
    connection_id: Option<&str>,
    owner_tx: &SyncSender<ServerInput>,
) {
    let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(connection_id) = connection_id {
        let device_id = state
            .broker
            .clients
            .iter()
            .find(|client| client.connection_id == connection_id)
            .map(|client| client.device_id.clone());
        state
            .broker
            .clients
            .retain(|client| client.connection_id != connection_id);
        if let Some(device) = state
            .disk
            .devices
            .iter_mut()
            .find(|device| Some(device.id.as_str()) == device_id.as_deref() && !device.revoked)
        {
            device.last_seen_at = Some(now());
            let _ = persist_disk_state(&state.root, &state.disk);
        }
        let _ = owner_tx.send(ServerInput::RemoteStatusChanged);
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum IncomingFrame {
    Pair {
        pairing_id: String,
        secret: String,
        device: DeviceHello,
    },
    Resume {
        device_id: String,
        device_token: String,
        #[serde(default)]
        server_epoch: Option<String>,
        #[serde(default)]
        after_revision: Option<u64>,
    },
    Command {
        command_id: String,
        action: String,
        #[serde(default)]
        params: Value,
    },
    Ack {
        through_revision: u64,
    },
    Ping,
    Pong,
}

#[derive(Debug, Deserialize)]
pub(super) struct DeviceHello {
    pub(super) display_name: String,
    pub(super) platform: String,
}

#[derive(Debug)]
pub(super) struct ResumeFrames {
    pub(super) welcome: Value,
    pub(super) replay: Vec<Value>,
    pub(super) latest_revision: u64,
}

pub(super) fn pair(
    shared: &Arc<Mutex<GatewayState>>,
    pairing_id: &str,
    secret: &str,
    device: DeviceHello,
) -> Result<(String, String, String, u64), (&'static str, &'static str)> {
    let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
    let timestamp = now();
    if !state.disk.enabled {
        return Err(("remote_disabled", "remote access is disabled"));
    }
    let Some(ticket) = state.pairing.as_mut() else {
        return Err(("pairing_unavailable", "pairing request is unavailable"));
    };
    if ticket.expires_at < timestamp {
        state.pairing = None;
        return Err(("pairing_expired", "pairing request expired"));
    }
    if ticket.id != pairing_id
        || !constant_time_eq(&ticket.secret_hash, &digest_secret(secret.as_bytes()))
    {
        ticket.attempts = ticket.attempts.saturating_add(1);
        if ticket.attempts >= PAIRING_MAX_ATTEMPTS {
            state.pairing = None;
        }
        return Err(("pairing_rejected", "pairing credentials were rejected"));
    }
    let profile_id = ticket.profile_id.clone();
    state.pairing = None;
    let device_id = random_id(16);
    let token = random_secret();
    state.disk.devices.push(StoredDevice {
        id: device_id.clone(),
        display_name: bounded_label(&device.display_name, "iPhone"),
        platform: bounded_label(&device.platform, "ios"),
        profile_id,
        token_hash: token_hash(&device_id, &token),
        paired_at: timestamp,
        last_seen_at: Some(timestamp),
        revoked: false,
    });
    if persist_disk_state(&state.root, &state.disk).is_err() {
        state.disk.devices.retain(|device| device.id != device_id);
        return Err(("pairing_storage_failed", "paired device could not be saved"));
    }
    Ok((
        device_id,
        token,
        state.broker.epoch.clone(),
        state.broker.revision,
    ))
}

pub(super) fn resume(
    shared: &Arc<Mutex<GatewayState>>,
    device_id: &str,
    token: &str,
    epoch: Option<&str>,
    after_revision: Option<u64>,
) -> Result<ResumeFrames, (&'static str, &'static str)> {
    let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
    let timestamp = now();
    if !state.disk.enabled {
        return Err(("remote_disabled", "remote access is disabled"));
    }
    let expected = token_hash(device_id, token);
    let Some(device) = state.disk.devices.iter_mut().find(|device| {
        device.id == device_id
            && !device.revoked
            && constant_time_eq(device.token_hash.as_bytes(), expected.as_bytes())
    }) else {
        return Err(("resume_rejected", "device credentials were rejected"));
    };
    device.last_seen_at = Some(timestamp);
    let _ = persist_disk_state(&state.root, &state.disk);
    let latest = state.broker.revision;
    let mut replay = Vec::new();
    if let Some(after) = after_revision {
        let first = state
            .broker
            .ring
            .front()
            .map_or(latest.saturating_add(1), |(_, _, event)| event.revision);
        if epoch != Some(state.broker.epoch.as_str())
            || after > latest
            || after.saturating_add(1) < first
        {
            replay.push(json!({"type":"resync_required","reason":"event_gap","server_epoch":state.broker.epoch,"latest_revision":latest}));
        } else {
            let profile_id = state
                .disk
                .devices
                .iter()
                .find(|device| device.id == device_id)
                .map(|device| device.profile_id.as_str());
            replay.extend(state.broker.ring.iter().filter_map(|(_, audience, event)| {
                if event.revision <= after {
                    return None;
                }
                if Some(audience.as_str()) == profile_id {
                    serde_json::to_value(event).ok()
                } else {
                    Some(json!({
                        "type":"event",
                        "server_epoch":state.broker.epoch,
                        "revision":event.revision,
                        "kind":"noop",
                        "payload":Value::Null,
                    }))
                }
            }));
        }
    }
    Ok(ResumeFrames {
        welcome: json!({"type":"welcome","server_epoch":state.broker.epoch,"latest_revision":latest,"profile_available":true}),
        replay,
        latest_revision: latest,
    })
}

pub(super) use super::commands::execute_command;

#[allow(
    clippy::too_many_arguments,
    reason = "all bounded per-connection channels must be registered atomically"
)]
pub(super) fn register_client(
    shared: &Arc<Mutex<GatewayState>>,
    device_id: &str,
    authenticated_revision: u64,
    sender: tokio_mpsc::Sender<QueuedFrame>,
    queued_bytes: Arc<AtomicUsize>,
    overflowed: Arc<AtomicBool>,
    overflow_notify: tokio::sync::watch::Sender<bool>,
    close: tokio::sync::watch::Sender<Option<ConnectionCloseReason>>,
    acknowledged: Arc<AtomicU64>,
) -> Result<String, (&'static str, &'static str)> {
    let connection_id = random_id(12);
    let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
    if !state.disk.enabled {
        return Err(("remote_disabled", "remote access is disabled"));
    }
    let Some(profile_id) = state
        .disk
        .devices
        .iter()
        .find(|device| device.id == device_id && !device.revoked)
        .map(|device| device.profile_id.clone())
    else {
        return Err(("device_revoked", "remote device was revoked"));
    };
    let other_clients = state
        .broker
        .clients
        .iter()
        .filter(|client| client.device_id != device_id)
        .count();
    if other_clients >= CLIENT_LIMIT {
        return Err(("server_busy", "remote connection limit was reached"));
    }
    let first_revision = state
        .broker
        .ring
        .front()
        .map_or(state.broker.revision.saturating_add(1), |(_, _, event)| {
            event.revision
        });
    if authenticated_revision.saturating_add(1) < first_revision {
        return Err(("event_gap", "remote events require a fresh snapshot"));
    }
    for (_, audience, event) in &state.broker.ring {
        if event.revision <= authenticated_revision {
            continue;
        }
        let value = if audience == &profile_id {
            serde_json::to_value(event).unwrap_or(Value::Null)
        } else {
            json!({
                "type":"event", "server_epoch":state.broker.epoch,
                "revision":event.revision, "kind":"noop", "payload":Value::Null,
            })
        };
        let bytes = serde_json::to_vec(&value).map_or(0, |encoded| encoded.len());
        let reserved = queued_bytes.fetch_add(bytes, Ordering::AcqRel);
        if reserved.saturating_add(bytes) > super::CLIENT_QUEUE_BYTES
            || sender.try_send(QueuedFrame { value, bytes }).is_err()
        {
            queued_bytes.fetch_sub(bytes, Ordering::AcqRel);
            return Err(("event_gap", "remote events require a fresh snapshot"));
        }
    }
    // Only retire the healthy old connection after the replacement has a
    // complete catch-up queue. A failed reconnect must not evict it.
    for client in &state.broker.clients {
        if client.device_id == device_id {
            let _ = client
                .close
                .send(Some(ConnectionCloseReason::ReplacedByNewConnection));
        }
    }
    state
        .broker
        .clients
        .retain(|client| client.device_id != device_id);
    state.broker.clients.push(ClientSink {
        connection_id: connection_id.clone(),
        device_id: device_id.to_string(),
        profile_id,
        sender,
        queued_bytes,
        overflowed,
        overflow_notify,
        close,
        _acknowledged: acknowledged,
    });
    Ok(connection_id)
}

pub(super) fn rollback_paired_device(shared: &Arc<Mutex<GatewayState>>, device_id: &str) {
    let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
    let Some(device) = state
        .disk
        .devices
        .iter_mut()
        .find(|device| device.id == device_id)
    else {
        return;
    };
    device.revoked = true;
    state
        .disk
        .receipts
        .retain(|receipt| receipt.device_id != device_id);
    for client in &state.broker.clients {
        if client.device_id == device_id {
            let _ = client
                .close
                .send(Some(ConnectionCloseReason::DeviceRevoked));
        }
    }
    state
        .broker
        .clients
        .retain(|client| client.device_id != device_id);
    if persist_disk_state(&state.root, &state.disk).is_err() {
        state.disk.enabled = false;
        state.last_error = Some("pairing_storage_failed".to_string());
    }
}

async fn send_error<S>(
    sink: &mut S,
    code: &str,
    message: &str,
) -> Result<(), tokio_tungstenite::tungstenite::Error>
where
    S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    send_with_timeout(
        sink,
        Message::Text(
            json!({"type":"error","error":{"code":code,"message":message}})
                .to_string()
                .into(),
        ),
    )
    .await
}

async fn send_with_timeout<S>(
    sink: &mut S,
    message: Message,
) -> Result<(), tokio_tungstenite::tungstenite::Error>
where
    S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    send_with_deadline(sink, message, Duration::from_secs(8)).await
}

pub(super) async fn send_with_deadline<S>(
    sink: &mut S,
    message: Message,
    deadline: Duration,
) -> Result<(), tokio_tungstenite::tungstenite::Error>
where
    S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    timeout(deadline, sink.send(message)).await.map_err(|_| {
        tokio_tungstenite::tungstenite::Error::Io(io::Error::new(
            io::ErrorKind::TimedOut,
            "remote websocket write timed out",
        ))
    })?
}

pub(super) fn token_hash(device_id: &str, token: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"pad.remote.v1 device token\0");
    digest.update(device_id.as_bytes());
    digest.update(b"\0");
    digest.update(token.as_bytes());
    hex(&digest.finalize())
}

fn bounded_label(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.chars().take(80).collect()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
