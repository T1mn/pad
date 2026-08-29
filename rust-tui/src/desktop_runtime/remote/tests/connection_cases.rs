use super::super::network::{
    execute_command, finish, load_or_create_tls, pair, register_client, resume,
    rollback_paired_device, send_with_deadline, DeviceHello,
};
use super::super::*;
use super::{cleanup, stored_device, test_root, test_state, ticket};
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio_tungstenite::tungstenite::protocol::Message;

type Registered = (
    tokio::sync::mpsc::Receiver<QueuedFrame>,
    tokio::sync::watch::Receiver<Option<ConnectionCloseReason>>,
    String,
);

pub(crate) fn disable_closes_without_revoking_and_reenable_accepts_the_same_token() {
    let root = test_root("disable-resume");
    let shared = test_state(&root);
    shared
        .lock()
        .unwrap()
        .disk
        .devices
        .push(stored_device("device-a", "a", "token"));
    let (mut events, mut close, _) = register(&shared, "device-a", 0).unwrap();
    let mut gateway = RemoteGateway {
        shared: Arc::clone(&shared),
        shutdown: Arc::new(AtomicBool::new(false)),
        network_thread: None,
        bonjour: None,
    };
    gateway.set_enabled(false, Some("a")).unwrap();
    // The broker deliberately drops its sender after writing the terminal
    // reason, so `has_changed` may report closure while the final value is
    // still available to the receiver.
    let _ = close.has_changed();
    assert_eq!(
        *close.borrow_and_update(),
        Some(ConnectionCloseReason::RemoteDisabled)
    );
    assert!(shared.lock().unwrap().broker.clients.is_empty());
    shared
        .lock()
        .unwrap()
        .publish("a", "invalidated", json!({"task_id":"private"}));
    assert!(
        events.try_recv().is_err(),
        "disabled client received an event"
    );

    gateway.set_enabled(true, Some("a")).unwrap();
    assert!(resume(&shared, "device-a", "token", None, None).is_ok());
    assert!(!shared.lock().unwrap().disk.devices[0].revoked);
    drop(gateway);
    cleanup(&root);
}

pub(crate) fn reconnect_replaces_the_old_device_connection_and_catches_up_atomically() {
    let root = test_root("connection-replace");
    let shared = test_state(&root);
    shared
        .lock()
        .unwrap()
        .disk
        .devices
        .push(stored_device("device-a", "a", "token"));
    let (mut old_events, mut old_close, _) = register(&shared, "device-a", 0).unwrap();
    let (mut new_events, _new_close, new_id) = register(&shared, "device-a", 0).unwrap();
    let _ = old_close.has_changed();
    assert_eq!(
        *old_close.borrow_and_update(),
        Some(ConnectionCloseReason::ReplacedByNewConnection)
    );
    {
        let mut state = shared.lock().unwrap();
        assert_eq!(state.broker.clients.len(), 1);
        assert_eq!(state.broker.clients[0].connection_id, new_id);
        state.publish("a", "invalidated", json!({"task_id":"a-task"}));
    }
    assert!(old_events.try_recv().is_err());
    assert_eq!(new_events.try_recv().unwrap().value["revision"], 1);

    let catchup_root = test_root("connection-catchup");
    let catchup = test_state(&catchup_root);
    {
        let mut state = catchup.lock().unwrap();
        state
            .disk
            .devices
            .push(stored_device("device-a", "a", "token"));
        state.publish("a", "invalidated", json!({"task_id":"before-register"}));
    }
    let (mut caught_up, _, _) = register(&catchup, "device-a", 0).unwrap();
    assert_eq!(
        caught_up.try_recv().unwrap().value["payload"]["task_id"],
        "before-register"
    );
    cleanup(&catchup_root);
    cleanup(&root);
}

pub(crate) fn registration_rechecks_disable_revoke_and_total_connection_limit() {
    let root = test_root("connection-limits");
    let shared = test_state(&root);
    {
        let mut state = shared.lock().unwrap();
        state
            .disk
            .devices
            .push(stored_device("device-a", "a", "token"));
        state.disk.enabled = false;
    }
    assert_eq!(
        register(&shared, "device-a", 0).unwrap_err().0,
        "remote_disabled"
    );
    {
        let mut state = shared.lock().unwrap();
        state.disk.enabled = true;
        state.disk.devices[0].revoked = true;
    }
    assert_eq!(
        register(&shared, "device-a", 0).unwrap_err().0,
        "device_revoked"
    );

    let mut held = Vec::new();
    {
        let mut state = shared.lock().unwrap();
        for index in 0..=CLIENT_LIMIT {
            state.disk.devices.push(stored_device(
                &format!("device-{index}"),
                "a",
                &format!("token-{index}"),
            ));
        }
    }
    for index in 0..CLIENT_LIMIT {
        held.push(register(&shared, &format!("device-{index}"), 0).unwrap());
    }
    assert_eq!(
        register(&shared, &format!("device-{CLIENT_LIMIT}"), 0)
            .unwrap_err()
            .0,
        "server_busy"
    );
    assert_eq!(shared.lock().unwrap().broker.clients.len(), CLIENT_LIMIT);
    drop(held);
    cleanup(&root);
}

pub(crate) fn stalled_websocket_writes_hit_the_deadline() {
    let mut sink = PendingSink;
    let error = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(send_with_deadline(
            &mut sink,
            Message::Text("ping".into()),
            std::time::Duration::from_millis(5),
        ))
        .unwrap_err();
    assert!(matches!(
        error,
        tokio_tungstenite::tungstenite::Error::Io(ref error)
            if error.kind() == std::io::ErrorKind::TimedOut
    ));
}

pub(crate) fn tls_identity_recovers_when_only_one_file_survives() {
    let root = test_root("tls-partial-recovery");
    let (_, original) = load_or_create_tls(&root).unwrap();
    std::fs::remove_file(root.join("tls-key.der")).unwrap();
    let (_, recovered) = load_or_create_tls(&root).unwrap();
    assert_ne!(original, recovered);
    assert!(root.join("tls-cert.der").is_file());
    assert!(root.join("tls-key.der").is_file());
    cleanup(&root);
}

pub(crate) fn failed_paired_delivery_leaves_no_resumable_orphan_device() {
    let root = test_root("pair-delivery-rollback");
    let shared = test_state(&root);
    shared.lock().unwrap().pairing = Some(PairingTicket {
        id: "pair-1".to_string(),
        secret_hash: digest_secret(b"secret"),
        profile_id: "a".to_string(),
        expires_at: now().saturating_add(60),
        attempts: 0,
    });
    let (device_id, token, _, _) = pair(
        &shared,
        "pair-1",
        "secret",
        DeviceHello {
            display_name: "Phone".to_string(),
            platform: "ios".to_string(),
        },
    )
    .unwrap();
    rollback_paired_device(&shared, &device_id);
    assert_eq!(
        resume(&shared, &device_id, &token, None, None)
            .unwrap_err()
            .0,
        "resume_rejected"
    );
    assert!(shared.lock().unwrap().disk.devices[0].revoked);
    cleanup(&root);
}

pub(crate) fn connection_finish_records_last_seen_without_reviving_revoked_devices() {
    let root = test_root("finish-last-seen");
    let shared = test_state(&root);
    let mut device = stored_device("device-a", "a", "token");
    device.last_seen_at = Some(1);
    shared.lock().unwrap().disk.devices.push(device);
    let (_events, _close, connection_id) = register(&shared, "device-a", 0).unwrap();
    let (owner_tx, _owner_rx) = std::sync::mpsc::sync_channel(2);
    finish(&shared, Some(&connection_id), &owner_tx);
    let seen = shared.lock().unwrap().disk.devices[0].last_seen_at.unwrap();
    assert!(seen > 1);
    assert!(shared.lock().unwrap().broker.clients.is_empty());

    let mut state = shared.lock().unwrap();
    state.disk.devices[0].revoked = true;
    state.disk.devices[0].last_seen_at = Some(7);
    drop(state);
    finish(&shared, Some("missing-connection"), &owner_tx);
    assert_eq!(shared.lock().unwrap().disk.devices[0].last_seen_at, Some(7));
    cleanup(&root);
}

pub(crate) fn failed_enable_and_revoke_persistence_roll_back_authoritative_state() {
    let root = test_root("setting-persist-rollback");
    let shared = test_state(&root);
    {
        let mut state = shared.lock().unwrap();
        state.disk.enabled = false;
        state
            .disk
            .devices
            .push(stored_device("device-a", "a", "token"));
        state.root = root.join("missing-parent").join("remote");
    }
    let mut gateway = RemoteGateway {
        shared: Arc::clone(&shared),
        shutdown: Arc::new(AtomicBool::new(false)),
        network_thread: None,
        bonjour: None,
    };
    assert!(gateway.set_enabled(true, Some("a")).is_err());
    assert!(!shared.lock().unwrap().disk.enabled);
    assert_eq!(
        resume(&shared, "device-a", "token", None, None)
            .unwrap_err()
            .0,
        "remote_disabled"
    );
    let (owner_tx, owner_rx) = std::sync::mpsc::sync_channel(1);
    let outcome = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(execute_command(
            &shared,
            &owner_tx,
            "device-a",
            "disabled-id",
            "history",
            json!({"task_id":"a-task"}),
        ));
    assert_eq!(outcome.error.unwrap().code, "remote_disabled");
    assert!(owner_rx.try_recv().is_err());

    {
        let mut state = shared.lock().unwrap();
        state.disk.enabled = true;
        state.pairing = Some(ticket("still-pairing", "secret"));
    }
    let (_events, close, _) = register(&shared, "device-a", 0).unwrap();
    assert!(gateway.set_enabled(false, Some("a")).is_err());
    {
        let state = shared.lock().unwrap();
        assert!(state.disk.enabled);
        assert_eq!(state.pairing.as_ref().unwrap().id, "still-pairing");
        assert_eq!(state.broker.clients.len(), 1);
    }
    assert!(matches!(close.has_changed(), Ok(false)));

    assert!(gateway.revoke_device("device-a", "a").is_err());
    assert!(!shared.lock().unwrap().disk.devices[0].revoked);
    cleanup(&root);
}

struct PendingSink;

impl futures_util::Sink<Message> for PendingSink {
    type Error = tokio_tungstenite::tungstenite::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Pending
    }

    fn start_send(self: std::pin::Pin<&mut Self>, _item: Message) -> Result<(), Self::Error> {
        unreachable!("pending sink never becomes ready")
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Pending
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

fn register(
    shared: &Arc<std::sync::Mutex<GatewayState>>,
    device_id: &str,
    revision: u64,
) -> Result<Registered, (&'static str, &'static str)> {
    let (sender, receiver) = tokio::sync::mpsc::channel(CLIENT_QUEUE_FRAMES);
    let (overflow, _overflow_receiver) = tokio::sync::watch::channel(false);
    let (close, close_receiver) = tokio::sync::watch::channel(None);
    let connection_id = register_client(
        shared,
        device_id,
        revision,
        sender,
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicBool::new(false)),
        overflow,
        close,
        Arc::new(AtomicU64::new(0)),
    )?;
    Ok((receiver, close_receiver, connection_id))
}
