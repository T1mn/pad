//! Authenticated LAN remote gateway owned by the Desktop bridge process.
//!
//! The gateway never constructs a `DesktopRuntime`. Remote commands cross a
//! bounded channel and are executed by the existing bridge owner loop.

use super::bridge::{RemoteOwnerRequest, ServerInput};
use base64::Engine as _;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::net::{TcpListener, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod commands;
mod network;
mod receipts;
mod tls;
mod wire;

pub(crate) use wire::{
    project_remote_result, remote_action_allowed, remote_action_mutates, RemoteCommandOutcome,
};

pub(crate) const REMOTE_SUBPROTOCOL: &str = "pad.remote.v1";
pub(crate) const MAX_REMOTE_FRAME_BYTES: usize = 1024 * 1024;
pub(crate) const OWNER_QUEUE_DEPTH: usize = 128;
const PAIRING_TTL_SECONDS: u64 = 120;
const PAIRING_MAX_ATTEMPTS: u8 = 3;
const RECEIPT_LIMIT: usize = 1024;
const RECEIPT_BYTES_LIMIT: usize = 16 * 1024 * 1024;
const RECEIPT_TTL_SECONDS: u64 = 24 * 60 * 60;
const EVENT_LIMIT: usize = 8192;
const EVENT_BYTES_LIMIT: usize = 16 * 1024 * 1024;
const CLIENT_QUEUE_FRAMES: usize = 256;
const CLIENT_QUEUE_BYTES: usize = 2 * 1024 * 1024;
const CLIENT_LIMIT: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StoredDevice {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) platform: String,
    pub(crate) profile_id: String,
    pub(crate) token_hash: String,
    pub(crate) paired_at: u64,
    #[serde(default)]
    pub(crate) last_seen_at: Option<u64>,
    #[serde(default)]
    pub(crate) revoked: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredReceipt {
    device_id: String,
    command_id: String,
    #[serde(default)]
    request_fingerprint: String,
    #[serde(default)]
    in_progress: bool,
    completed_at: u64,
    outcome: RemoteCommandOutcome,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DiskState {
    version: u32,
    enabled: bool,
    #[serde(default)]
    listen_port: Option<u16>,
    #[serde(default)]
    devices: Vec<StoredDevice>,
    #[serde(default)]
    receipts: Vec<StoredReceipt>,
}

impl Default for DiskState {
    fn default() -> Self {
        Self {
            version: 1,
            enabled: false,
            listen_port: None,
            devices: Vec::new(),
            receipts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct PairingTicket {
    id: String,
    secret_hash: [u8; 32],
    profile_id: String,
    expires_at: u64,
    attempts: u8,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct RemoteEvent {
    #[serde(rename = "type")]
    frame_type: &'static str,
    server_epoch: String,
    revision: u64,
    kind: String,
    payload: Value,
}

#[derive(Debug)]
struct QueuedFrame {
    value: Value,
    bytes: usize,
}

#[derive(Debug)]
struct ClientSink {
    connection_id: String,
    device_id: String,
    profile_id: String,
    sender: tokio::sync::mpsc::Sender<QueuedFrame>,
    queued_bytes: Arc<AtomicUsize>,
    overflowed: Arc<AtomicBool>,
    overflow_notify: tokio::sync::watch::Sender<bool>,
    close: tokio::sync::watch::Sender<Option<ConnectionCloseReason>>,
    _acknowledged: Arc<std::sync::atomic::AtomicU64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConnectionCloseReason {
    RemoteDisabled,
    DeviceRevoked,
    ReplacedByNewConnection,
}

#[derive(Debug)]
struct EventBroker {
    epoch: String,
    revision: u64,
    ring_bytes: usize,
    ring: VecDeque<(usize, String, RemoteEvent)>,
    clients: Vec<ClientSink>,
}

#[derive(Debug)]
struct GatewayState {
    root: PathBuf,
    endpoint: String,
    fingerprint: String,
    display_name: String,
    disk: DiskState,
    pairing: Option<PairingTicket>,
    last_error: Option<String>,
    broker: EventBroker,
    ephemeral_receipts: VecDeque<StoredReceipt>,
    inflight: HashMap<String, InflightCommand>,
}

#[derive(Debug)]
struct InflightCommand {
    request_fingerprint: String,
    waiters: Vec<std::sync::mpsc::Sender<RemoteCommandOutcome>>,
}

pub(crate) struct RemoteGateway {
    shared: Arc<Mutex<GatewayState>>,
    shutdown: Arc<AtomicBool>,
    network_thread: Option<JoinHandle<()>>,
    bonjour: Option<Child>,
}

impl RemoteGateway {
    pub(crate) fn start(data_root: &Path, owner_tx: SyncSender<ServerInput>) -> io::Result<Self> {
        let root = data_root.join("v1").join("remote");
        crate::paths::base::ensure_private_dir(&root)?;
        let mut disk = load_disk_state(&root)?;
        prune_receipts(&mut disk, now());
        let (tls, fingerprint) = network::load_or_create_tls(&root)?;
        let requested_port = disk.listen_port.unwrap_or(0);
        let listener = TcpListener::bind(("0.0.0.0", requested_port)).map_err(|error| {
            if requested_port == 0 {
                error
            } else {
                io::Error::new(
                    error.kind(),
                    format!("persisted PAD remote port {requested_port} is unavailable"),
                )
            }
        })?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        disk.listen_port = Some(port);
        persist_disk_state(&root, &disk)?;
        let endpoint = format!("wss://{}:{port}", advertised_host());
        let display_name = mac_display_name();
        let epoch = random_id(16);
        let shared = Arc::new(Mutex::new(GatewayState {
            root,
            endpoint,
            fingerprint,
            display_name,
            disk,
            pairing: None,
            last_error: None,
            broker: EventBroker {
                epoch,
                revision: 0,
                ring_bytes: 0,
                ring: VecDeque::new(),
                clients: Vec::new(),
            },
            ephemeral_receipts: VecDeque::new(),
            inflight: HashMap::new(),
        }));
        let shutdown = Arc::new(AtomicBool::new(false));
        let network_thread = Some(network::spawn(
            listener,
            tls,
            Arc::clone(&shared),
            owner_tx,
            Arc::clone(&shutdown),
        )?);
        let mut gateway = Self {
            shared,
            shutdown,
            network_thread,
            bonjour: None,
        };
        if gateway.enabled() {
            gateway.start_bonjour(port);
        }
        Ok(gateway)
    }

    pub(crate) fn status_value(&self, profile_id: Option<&str>) -> Value {
        let state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        json!({ "remote": public_status(&state, profile_id) })
    }

    pub(crate) fn set_enabled(
        &mut self,
        enabled: bool,
        profile_id: Option<&str>,
    ) -> io::Result<Value> {
        let port;
        {
            let mut state = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let mut candidate = state.disk.clone();
            candidate.enabled = enabled;
            if let Err(error) = persist_disk_state(&state.root, &candidate) {
                state.last_error = Some("remote_persistence_failed".to_string());
                return Err(error);
            }
            state.disk = candidate;
            if !enabled {
                state.pairing = None;
                for client in &state.broker.clients {
                    let _ = client
                        .close
                        .send(Some(ConnectionCloseReason::RemoteDisabled));
                }
                state.broker.clients.clear();
            }
            state.last_error = None;
            port = endpoint_port(&state.endpoint).unwrap_or(0);
        }
        if enabled {
            self.start_bonjour(port);
        } else {
            self.stop_bonjour();
        }
        Ok(self.status_value(profile_id))
    }

    pub(crate) fn pair_begin(&mut self, profile_id: &str) -> io::Result<Value> {
        if !self.enabled() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "remote access is disabled",
            ));
        }
        let secret = random_secret();
        let pairing_id = random_id(16);
        let expires_at = now().saturating_add(PAIRING_TTL_SECONDS);
        let qr_payload;
        {
            let mut state = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            qr_payload = pairing_uri(&state.endpoint, &state.fingerprint, &pairing_id, &secret);
            state.pairing = Some(PairingTicket {
                id: pairing_id.clone(),
                secret_hash: digest_secret(secret.as_bytes()),
                profile_id: profile_id.to_string(),
                expires_at,
                attempts: 0,
            });
        }
        Ok(json!({
            "pairing": {
                "pairing_id": pairing_id,
                "qr_payload": qr_payload,
                "expires_at": expires_at,
            }
        }))
    }

    pub(crate) fn pair_cancel(&mut self, pairing_id: &str, profile_id: &str) -> io::Result<Value> {
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let matches = state
            .pairing
            .as_ref()
            .is_some_and(|ticket| ticket.id == pairing_id && ticket.profile_id == profile_id);
        if !matches {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "pairing was not found",
            ));
        }
        state.pairing = None;
        Ok(json!({ "remote": public_status(&state, Some(profile_id)) }))
    }

    pub(crate) fn revoke_device(&mut self, device_id: &str, profile_id: &str) -> io::Result<Value> {
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(index) = state
            .disk
            .devices
            .iter()
            .position(|device| device.id == device_id && device.profile_id == profile_id)
        else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "remote device was not found",
            ));
        };
        let previous_device = state.disk.devices[index].clone();
        let previous_receipts = state.disk.receipts.clone();
        state.disk.devices[index].revoked = true;
        state
            .disk
            .receipts
            .retain(|receipt| receipt.device_id != device_id);
        if let Err(error) = persist_disk_state(&state.root, &state.disk) {
            state.disk.devices[index] = previous_device;
            state.disk.receipts = previous_receipts;
            return Err(error);
        }
        state.broker.clients.retain(|client| {
            if client.device_id == device_id {
                let _ = client
                    .close
                    .send(Some(ConnectionCloseReason::DeviceRevoked));
                false
            } else {
                true
            }
        });
        Ok(json!({ "remote": public_status(&state, Some(profile_id)) }))
    }

    pub(crate) fn device_profile(&self, device_id: &str) -> Option<String> {
        let state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !state.disk.enabled {
            return None;
        }
        state
            .disk
            .devices
            .iter()
            .find(|device| device.id == device_id && !device.revoked)
            .map(|device| device.profile_id.clone())
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled()
    }

    pub(crate) fn publish_profile_event(&self, profile_id: &str, kind: &str, payload: Value) {
        self.shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .publish(profile_id, kind, payload);
    }

    pub(crate) fn remote_changed(&self, action: &str, profile_id: Option<&str>) -> Value {
        let state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let remote = public_status(&state, profile_id);
        json!({
            "action": action,
            "remote": remote,
            "has_online_remote": !state.broker.clients.is_empty(),
        })
    }

    fn enabled(&self) -> bool {
        self.shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .disk
            .enabled
    }

    fn start_bonjour(&mut self, port: u16) {
        if self.bonjour.is_some() || port == 0 {
            return;
        }
        self.bonjour = Command::new("/usr/bin/dns-sd")
            .args([
                "-R",
                "PAD Desktop",
                "_pad-remote._tcp",
                "local",
                &port.to_string(),
                "v=1",
                "proto=pad.remote.v1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
    }

    fn stop_bonjour(&mut self) {
        if let Some(mut child) = self.bonjour.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for RemoteGateway {
    fn drop(&mut self) {
        self.stop_bonjour();
        self.shutdown.store(true, Ordering::Release);
        if let Some(thread) = self.network_thread.take() {
            let _ = thread.join();
        }
    }
}

impl GatewayState {
    fn publish(&mut self, profile_id: &str, kind: &str, payload: Value) {
        self.broker.revision = self.broker.revision.saturating_add(1);
        let mut event = RemoteEvent {
            frame_type: "event",
            server_epoch: self.broker.epoch.clone(),
            revision: self.broker.revision,
            kind: kind.to_string(),
            payload,
        };
        let mut value = serde_json::to_value(&event).unwrap_or(Value::Null);
        let mut bytes = serde_json::to_vec(&value).map_or(0, |encoded| encoded.len());
        if bytes > MAX_REMOTE_FRAME_BYTES {
            let task_id = event
                .payload
                .get("task_id")
                .and_then(Value::as_str)
                .filter(|task_id| task_id.len() <= 256)
                .map(str::to_string);
            event.kind = "invalidated".to_string();
            event.payload = task_id.map_or_else(
                || json!({"reason":"payload_too_large"}),
                |task_id| json!({"reason":"payload_too_large","task_id":task_id}),
            );
            value = serde_json::to_value(&event).unwrap_or(Value::Null);
            bytes = serde_json::to_vec(&value).map_or(0, |encoded| encoded.len());
        }
        self.broker.ring_bytes = self.broker.ring_bytes.saturating_add(bytes);
        self.broker
            .ring
            .push_back((bytes, profile_id.to_string(), event));
        while self.broker.ring.len() > EVENT_LIMIT || self.broker.ring_bytes > EVENT_BYTES_LIMIT {
            if let Some((removed, _, _)) = self.broker.ring.pop_front() {
                self.broker.ring_bytes = self.broker.ring_bytes.saturating_sub(removed);
            }
        }
        self.broker.clients.retain(|client| {
            let (client_value, client_bytes) = if profile_id != client.profile_id {
                let noop = RemoteEvent {
                    frame_type: "event",
                    server_epoch: self.broker.epoch.clone(),
                    revision: self.broker.revision,
                    kind: "noop".to_string(),
                    payload: Value::Null,
                };
                let value = serde_json::to_value(noop).unwrap_or(Value::Null);
                let bytes = serde_json::to_vec(&value).map_or(0, |encoded| encoded.len());
                (value, bytes)
            } else {
                (value.clone(), bytes)
            };
            let queued = client.queued_bytes.load(Ordering::Acquire);
            if queued.saturating_add(client_bytes) > CLIENT_QUEUE_BYTES {
                client.overflowed.store(true, Ordering::Release);
                let _ = client.overflow_notify.send(true);
                return false;
            }
            client
                .queued_bytes
                .fetch_add(client_bytes, Ordering::AcqRel);
            match client.sender.try_send(QueuedFrame {
                value: client_value,
                bytes: client_bytes,
            }) {
                Ok(()) => true,
                Err(_) => {
                    client
                        .queued_bytes
                        .fetch_sub(client_bytes, Ordering::AcqRel);
                    client.overflowed.store(true, Ordering::Release);
                    let _ = client.overflow_notify.send(true);
                    false
                }
            }
        });
    }
}

fn public_status(state: &GatewayState, profile_id: Option<&str>) -> Value {
    let devices = state
        .disk
        .devices
        .iter()
        .filter(|device| {
            !device.revoked && profile_id.is_some_and(|profile_id| device.profile_id == profile_id)
        })
        .map(|device| {
            json!({
                "id": device.id,
                "display_name": device.display_name,
                "platform": device.platform,
                "online": state.broker.clients.iter().any(|client| {
                    client.device_id == device.id && client.profile_id == device.profile_id
                }),
                "paired_at": device.paired_at,
                "last_seen_at": device.last_seen_at,
            })
        })
        .collect::<Vec<_>>();
    let state_name = if !state.disk.enabled {
        "disabled"
    } else if state.last_error.is_some() {
        "failed"
    } else {
        "ready"
    };
    let mut value = json!({
        "enabled": state.disk.enabled,
        "state": state_name,
        "display_name": state.display_name,
        "active_connections": state
            .broker
            .clients
            .iter()
            .filter(|client| profile_id.is_some_and(|profile_id| client.profile_id == profile_id))
            .count(),
        "devices": devices,
        "updated_at": now(),
    });
    if let Some(error) = &state.last_error {
        value["error_code"] = Value::String(error.clone());
    }
    value
}

fn load_disk_state(root: &Path) -> io::Result<DiskState> {
    let path = root.join("state.json");
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(DiskState::default()),
        Err(error) => Err(error),
    }
}

fn persist_disk_state(root: &Path, state: &DiskState) -> io::Result<()> {
    let encoded = serde_json::to_vec(state)?;
    let path = root.join("state.json");
    let temporary = root.join(format!(".state-{}.tmp", random_id(8)));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    fs::rename(&temporary, &path)?;
    crate::paths::base::harden_private_tree(root)
}

fn prune_receipts(state: &mut DiskState, timestamp: u64) {
    state
        .receipts
        .retain(|receipt| timestamp.saturating_sub(receipt.completed_at) <= RECEIPT_TTL_SECONDS);
    if state.receipts.len() > RECEIPT_LIMIT {
        let remove = state.receipts.len() - RECEIPT_LIMIT;
        state.receipts.drain(..remove);
    }
    while receipt_storage_bytes(&state.receipts) > RECEIPT_BYTES_LIMIT {
        if state.receipts.is_empty() {
            break;
        }
        state.receipts.remove(0);
    }
}

fn receipt_storage_bytes(receipts: &[StoredReceipt]) -> usize {
    receipts.iter().fold(0, |total, receipt| {
        total.saturating_add(
            serde_json::to_vec(receipt)
                .map_or(RECEIPT_BYTES_LIMIT.saturating_add(1), |value| value.len()),
        )
    })
}

fn advertised_host() -> String {
    if let Ok(output) = Command::new("/usr/sbin/scutil")
        .args(["--get", "LocalHostName"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if output.status.success()
            && !name.is_empty()
            && name.len() <= 63
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && !name.starts_with('-')
            && !name.ends_with('-')
        {
            return format!("{name}.local");
        }
    }
    UdpSocket::bind(("0.0.0.0", 0))
        .and_then(|socket| {
            socket.connect(("192.0.2.1", 9))?;
            socket.local_addr()
        })
        .map(|address| address.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn mac_display_name() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "PAD Desktop on Mac".to_string())
}

fn endpoint_port(endpoint: &str) -> Option<u16> {
    endpoint.rsplit_once(':')?.1.parse().ok()
}

fn pairing_uri(endpoint: &str, fingerprint: &str, id: &str, secret: &str) -> String {
    format!(
        "pad://remote/pair?v=1&endpoint={}&fingerprint={fingerprint}&pairing_id={id}&secret={secret}",
        percent_encode(endpoint)
    )
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn random_secret() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random_id(bytes: usize) -> String {
    let mut value = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut value);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value)
}

fn digest_secret(secret: &[u8]) -> [u8; 32] {
    Sha256::digest(secret).into()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

#[cfg(test)]
pub(crate) mod tests;
