use super::network::{execute_command, pair, resume, token_hash, DeviceHello};
use super::*;
use crate::desktop_runtime::bridge::ServerInput;
use crate::pad_store::DesktopUiState;
use crate::permission_policy::{Profile, Task};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub(crate) mod connection_cases;
pub(crate) mod pending_cases;
pub(crate) mod receipt_cases;

pub(crate) fn remote_action_allowlist_denies_privileged_and_poll_routes() {
    for action in [
        "poll",
        "auth_begin",
        "auth_status",
        "set_profile",
        "set_ui_state",
        "terminal_open",
        "provider_status",
    ] {
        assert!(
            !remote_action_allowed(action),
            "{action} must stay local-only"
        );
    }
    for action in ["bootstrap", "history", "prompt", "respond_ui"] {
        assert!(
            remote_action_allowed(action),
            "{action} should be remotely available"
        );
    }
}

pub(crate) fn pairing_expires_is_one_time_and_locks_after_three_failures() {
    let root = test_root("pairing");
    let shared = test_state(&root);
    shared.lock().unwrap().pairing = Some(PairingTicket {
        id: "expired".to_string(),
        secret_hash: digest_secret(b"secret"),
        profile_id: "a".to_string(),
        expires_at: now().saturating_sub(1),
        attempts: 0,
    });
    assert_eq!(
        pair(&shared, "expired", "secret", device()).unwrap_err().0,
        "pairing_expired"
    );
    shared.lock().unwrap().pairing = Some(ticket("attempts", "right"));
    for _ in 0..3 {
        assert_eq!(
            pair(&shared, "attempts", "wrong", device()).unwrap_err().0,
            "pairing_rejected"
        );
    }
    assert!(shared.lock().unwrap().pairing.is_none());
    shared.lock().unwrap().pairing = Some(ticket("once", "right"));
    let paired = pair(&shared, "once", "right", device()).unwrap();
    assert_eq!(paired.1.len(), 43);
    assert_eq!(
        pair(&shared, "once", "right", device()).unwrap_err().0,
        "pairing_unavailable"
    );
    cleanup(&root);
}

pub(crate) fn device_tokens_are_persisted_only_as_context_bound_hashes() {
    let hash = token_hash("device-a", "super-secret-token");
    assert_eq!(hash.len(), 64);
    assert_eq!(hash, token_hash("device-a", "super-secret-token"));
    assert_ne!(hash, token_hash("device-b", "super-secret-token"));
    assert_ne!(hash, token_hash("device-a", "other-token"));
    let encoded = serde_json::to_string(&StoredDevice {
        id: "device-a".to_string(),
        display_name: "Phone".to_string(),
        platform: "ios".to_string(),
        profile_id: "a".to_string(),
        token_hash: hash,
        paired_at: 1,
        last_seen_at: None,
        revoked: false,
    })
    .unwrap();
    assert!(!encoded.contains("super-secret-token"));
    let uri = pairing_uri(
        "wss://Tim-Mac.local:43827",
        &"ab".repeat(32),
        "pairing-id",
        "one-time-secret",
    );
    assert!(uri.starts_with("pad://remote/pair?v=1&endpoint=wss%3A%2F%2F"));
    assert!(uri.contains("fingerprint="));
    assert!(uri.contains("pairing_id=pairing-id&secret=one-time-secret"));
}

pub(crate) fn disabled_gateway_rejects_pair_resume_and_command_at_authoritative_entrypoints() {
    let root = test_root("disabled");
    let shared = test_state(&root);
    let token = "device-token";
    {
        let mut state = shared.lock().unwrap();
        state.disk.enabled = false;
        state.pairing = Some(ticket("pair", "secret"));
        state
            .disk
            .devices
            .push(stored_device("device-a", "a", token));
    }
    assert_eq!(
        pair(&shared, "pair", "secret", device()).unwrap_err().0,
        "remote_disabled"
    );
    assert_eq!(
        resume(&shared, "device-a", token, Some("epoch-1"), Some(0))
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
            "disabled-command",
            "history",
            json!({"task_id":"a-task"}),
        ));
    assert_eq!(outcome.error.unwrap().code, "remote_disabled");
    assert!(owner_rx.try_recv().is_err());
    cleanup(&root);
}

pub(crate) fn concurrent_duplicate_command_has_one_owner_execution_and_one_receipt() {
    let root = test_root("receipt");
    let shared = test_state(&root);
    shared
        .lock()
        .unwrap()
        .disk
        .devices
        .push(stored_device("device-a", "a", "token"));
    let (owner_tx, owner_rx) = std::sync::mpsc::sync_channel(8);
    let executions = Arc::new(AtomicU64::new(0));
    let counter = Arc::clone(&executions);
    let owner = std::thread::spawn(move || {
        let ServerInput::Remote(request) = owner_rx.recv_timeout(Duration::from_secs(2)).unwrap()
        else {
            panic!("expected remote command")
        };
        counter.fetch_add(1, Ordering::AcqRel);
        std::thread::sleep(Duration::from_millis(40));
        request
            .response
            .send(RemoteCommandOutcome {
                ok: true,
                result: Some(json!({"accepted":true})),
                error: None,
            })
            .unwrap();
        assert!(owner_rx.recv_timeout(Duration::from_millis(120)).is_err());
    });
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (left, right) = runtime.block_on(async {
        tokio::join!(
            execute_command(
                &shared,
                &owner_tx,
                "device-a",
                "same-command",
                "prompt",
                json!({"task_id":"a","prompt":"hello"}),
            ),
            execute_command(
                &shared,
                &owner_tx,
                "device-a",
                "same-command",
                "prompt",
                json!({"task_id":"a","prompt":"hello"}),
            )
        )
    });
    owner.join().unwrap();
    assert_eq!(left, right);
    assert_eq!(executions.load(Ordering::Acquire), 1);
    assert_eq!(shared.lock().unwrap().disk.receipts.len(), 1);
    cleanup(&root);
}

pub(crate) fn event_replay_requires_epoch_and_filters_the_bound_profile() {
    let root = test_root("replay");
    let shared = test_state(&root);
    let token = "device-token";
    {
        let mut state = shared.lock().unwrap();
        state
            .disk
            .devices
            .push(stored_device("device-a", "a", token));
        state.publish("a", "invalidated", json!({"task_id":"a-task"}));
        state.publish("b", "invalidated", json!({"task_id":"b-task"}));
    }
    let epoch = shared.lock().unwrap().broker.epoch.clone();
    let replayed = resume(&shared, "device-a", token, Some(&epoch), Some(0)).unwrap();
    assert_eq!(replayed.replay.len(), 2);
    assert_eq!(replayed.replay[0]["payload"]["task_id"], "a-task");
    assert_eq!(replayed.replay[1]["kind"], "noop");
    assert_eq!(replayed.replay[0]["revision"], 1);
    assert_eq!(replayed.replay[1]["revision"], 2);
    assert!(!serde_json::to_string(&replayed.replay)
        .unwrap()
        .contains("b-task"));
    let resync = resume(&shared, "device-a", token, Some("old-epoch"), Some(0)).unwrap();
    assert_eq!(resync.replay[0]["type"], "resync_required");
    let ahead = resume(&shared, "device-a", token, Some(&epoch), Some(99)).unwrap();
    assert_eq!(ahead.replay[0]["type"], "resync_required");
    cleanup(&root);
}

pub(crate) fn interleaved_profile_events_keep_contiguous_noop_cursors_without_leaks() {
    let root = test_root("profile-events");
    let shared = test_state(&root);
    let (sender_a, mut receiver_a) = tokio::sync::mpsc::channel(8);
    let (sender_b, mut receiver_b) = tokio::sync::mpsc::channel(8);
    {
        let mut state = shared.lock().unwrap();
        state.broker.clients.push(test_client("a", sender_a));
        state.broker.clients.push(test_client("b", sender_b));
        state.publish("a", "invalidated", json!({"task_id":"a-secret"}));
        state.publish("b", "invalidated", json!({"task_id":"b-secret"}));
    }
    let a_frames = [
        receiver_a.try_recv().unwrap(),
        receiver_a.try_recv().unwrap(),
    ];
    let b_frames = [
        receiver_b.try_recv().unwrap(),
        receiver_b.try_recv().unwrap(),
    ];
    assert_eq!(a_frames[0].value["revision"], 1);
    assert_eq!(a_frames[1].value["revision"], 2);
    assert_eq!(b_frames[0].value["revision"], 1);
    assert_eq!(b_frames[1].value["revision"], 2);
    assert_eq!(a_frames[1].value["kind"], "noop");
    assert_eq!(b_frames[0].value["kind"], "noop");
    assert!(!a_frames
        .iter()
        .any(|frame| frame.value.to_string().contains("b-secret")));
    assert!(!b_frames
        .iter()
        .any(|frame| frame.value.to_string().contains("a-secret")));
    cleanup(&root);
}

pub(crate) fn oversized_event_becomes_small_invalidation_before_ring_and_client_queue() {
    let root = test_root("oversized-event");
    let shared = test_state(&root);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    {
        let mut state = shared.lock().unwrap();
        state.broker.clients.push(test_client("a", sender));
        state.publish(
            "a",
            "task_output",
            json!({
                "task_id":"background-task",
                "messages":["x".repeat(MAX_REMOTE_FRAME_BYTES + 1024)]
            }),
        );
        assert!(state.broker.ring.front().unwrap().0 < MAX_REMOTE_FRAME_BYTES);
        assert_eq!(state.broker.ring.front().unwrap().2.kind, "invalidated");
        assert_eq!(
            state.broker.ring.front().unwrap().2.payload["task_id"],
            "background-task"
        );
    }
    let frame = receiver.try_recv().unwrap();
    assert!(frame.bytes < MAX_REMOTE_FRAME_BYTES);
    assert_eq!(frame.value["kind"], "invalidated");
    assert_eq!(frame.value["payload"]["task_id"], "background-task");
    assert!(!frame.value.to_string().contains(&"x".repeat(1024)));
    cleanup(&root);
}

pub(crate) fn read_only_remote_actions_do_not_emit_invalidation_loops() {
    let root = test_root("read-only-invalidations");
    let shared = test_state(&root);
    shared
        .lock()
        .unwrap()
        .disk
        .devices
        .push(stored_device("device-a", "a", "token"));
    let mut runtime = super::super::DesktopRuntime::in_memory().unwrap();
    runtime
        .create_profile(Profile {
            id: "a".to_string(),
            name: "A".to_string(),
            ..Profile::default()
        })
        .unwrap();
    runtime
        .create_task(Task {
            id: "a-task".to_string(),
            profile_id: "a".to_string(),
            ..Task::default()
        })
        .unwrap();
    runtime.remote_gateway = Some(RemoteGateway {
        shared: Arc::clone(&shared),
        shutdown: Arc::new(AtomicBool::new(false)),
        network_thread: None,
        bonjour: None,
    });

    let history =
        runtime.execute_remote_command_for_test("device-a", "history", json!({"task_id":"a-task"}));
    assert!(history.ok);
    assert_eq!(shared.lock().unwrap().broker.revision, 0);

    let mutation = runtime.execute_remote_command_for_test(
        "device-a",
        "set_task",
        json!({"task_id":"a-task","unread":true}),
    );
    assert!(mutation.ok);
    assert_eq!(shared.lock().unwrap().broker.revision, 1);
    assert_eq!(shared.lock().unwrap().broker.ring.len(), 1);

    let snapshot = runtime.execute_remote_command_for_test(
        "device-a",
        "runtime_snapshot",
        json!({"task_id":"a-task"}),
    );
    assert!(snapshot.ok);
    assert_eq!(shared.lock().unwrap().broker.revision, 1);
    drop(runtime);
    cleanup(&root);
}

pub(crate) fn local_status_and_revocation_are_profile_scoped() {
    let root = test_root("status-scope");
    let shared = test_state(&root);
    {
        let mut state = shared.lock().unwrap();
        state
            .disk
            .devices
            .push(stored_device("device-a", "a", "a-token"));
        state
            .disk
            .devices
            .push(stored_device("device-b", "b", "b-token"));
    }
    let mut gateway = RemoteGateway {
        shared,
        shutdown: Arc::new(AtomicBool::new(false)),
        network_thread: None,
        bonjour: None,
    };
    let a_status = gateway.status_value(Some("a"));
    assert_eq!(a_status["remote"]["devices"].as_array().unwrap().len(), 1);
    assert_eq!(a_status["remote"]["devices"][0]["id"], "device-a");
    assert_eq!(
        gateway.revoke_device("device-b", "a").unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    gateway.revoke_device("device-a", "a").unwrap();
    assert!(gateway.status_value(Some("a"))["remote"]["devices"]
        .as_array()
        .unwrap()
        .is_empty());
    drop(gateway);
    cleanup(&root);
}

pub(crate) fn slow_client_queue_is_bounded_and_marked_for_resync() {
    let root = test_root("slow");
    let shared = test_state(&root);
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    let (close, _close_rx) = tokio::sync::watch::channel(None);
    let (overflow_notify, mut overflow_rx) = tokio::sync::watch::channel(false);
    let overflowed = Arc::new(AtomicBool::new(false));
    {
        let mut state = shared.lock().unwrap();
        state.broker.clients.push(ClientSink {
            connection_id: "slow".to_string(),
            device_id: "device-a".to_string(),
            profile_id: "a".to_string(),
            sender,
            queued_bytes: Arc::new(AtomicUsize::new(CLIENT_QUEUE_BYTES)),
            overflowed: Arc::clone(&overflowed),
            overflow_notify,
            close,
            _acknowledged: Arc::new(AtomicU64::new(0)),
        });
        state.publish("a", "task_output", json!({"delta":"x"}));
        assert!(state.broker.clients.is_empty());
    }
    assert!(overflowed.load(Ordering::Acquire));
    let _ = overflow_rx.has_changed();
    assert!(*overflow_rx.borrow_and_update());
    cleanup(&root);
}

pub(crate) fn remote_projection_removes_paths_credentials_provider_and_raw_stderr() {
    let projected = project_remote_result(json!({
        "task": {"id":"task-a","cwd":"/tmp/work","session_id":"secret-session"},
        "backend": {"provider_authentication":"ready","stderr":"raw failure"},
        "token":"secret",
        "messages":[{"text":"safe"}],
    }));
    let encoded = projected.to_string();
    for forbidden in [
        "cwd",
        "session_id",
        "provider_authentication",
        "stderr",
        "token",
    ] {
        assert!(!encoded.contains(forbidden));
    }
    assert_eq!(projected["messages"][0]["text"], "safe");
}

pub(crate) fn profile_bound_remote_actions_ignore_mac_selection_and_reject_other_tasks() {
    let mut runtime = super::super::DesktopRuntime::in_memory().unwrap();
    for id in ["a", "b"] {
        runtime
            .create_profile(Profile {
                id: id.to_string(),
                name: id.to_uppercase(),
                ..Profile::default()
            })
            .unwrap();
        runtime
            .create_task(Task {
                id: format!("{id}-task"),
                profile_id: id.to_string(),
                title: format!("Task {id}"),
                ..Task::default()
            })
            .unwrap();
    }
    runtime
        .set_desktop_ui_state(DesktopUiState {
            active_profile_id: Some("b".to_string()),
            selected_task_id: Some("b-task".to_string()),
            ..DesktopUiState::default()
        })
        .unwrap();
    let own = runtime
        .execute_profile_remote_action_for_test(
            "a",
            "runtime_snapshot",
            json!({"task_id":"a-task"}),
        )
        .unwrap();
    assert_eq!(own["task_id"], "a-task");
    let other = runtime
        .execute_profile_remote_action_for_test(
            "a",
            "runtime_snapshot",
            json!({"task_id":"b-task"}),
        )
        .unwrap_err();
    assert_eq!(other.code, "task_not_found");
    assert_eq!(
        runtime
            .desktop_ui_state()
            .unwrap()
            .active_profile_id
            .as_deref(),
        Some("b")
    );
}

pub(crate) fn owner_pump_broadcasts_pi_output_without_any_renderer_poll() {
    let mut runtime = super::super::DesktopRuntime::in_memory().unwrap();
    crate::desktop_runtime::tests::configure_fake_pi(
        &mut runtime,
        &[json!({
            "type":"message_update",
            "generation":1,
            "sequence":1,
            "delta":"live"
        })],
    );
    runtime
        .create_profile(Profile {
            id: "a".to_string(),
            name: "A".to_string(),
            ..Profile::default()
        })
        .unwrap();
    runtime
        .create_task(Task {
            id: "a-task".to_string(),
            profile_id: "a".to_string(),
            cwd: std::env::temp_dir(),
            ..Task::default()
        })
        .unwrap();
    let (owner_tx, _owner_rx) = std::sync::mpsc::sync_channel(8);
    runtime.attach_remote_gateway(owner_tx);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
    runtime
        .remote_gateway
        .as_ref()
        .unwrap()
        .shared
        .lock()
        .unwrap()
        .broker
        .clients
        .push(test_client("a", sender));
    runtime.start_task("a-task").unwrap();
    let mut frame = None;
    let mut sequence = 0;
    for _ in 0..40 {
        let mut local_stdout = Vec::new();
        crate::desktop_runtime::bridge::remote_events::pump_runtime_tasks(
            &mut runtime,
            &mut local_stdout,
            false,
            &mut sequence,
        )
        .unwrap();
        assert!(local_stdout.is_empty());
        if let Ok(value) = receiver.try_recv() {
            frame = Some(value);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let frame = frame.expect("central owner pump should publish Pi output");
    assert_eq!(frame.value["kind"], "task_output");
    assert_eq!(frame.value["payload"]["task_id"], "a-task");
    runtime.stop_task("a-task").unwrap();
}

pub(crate) fn persisted_listener_port_is_reused_after_gateway_restart() {
    let data_root = crate::test_support::temp_path("pad-remote", "port-reuse");
    let (owner_tx, _owner_rx) = std::sync::mpsc::sync_channel(4);
    let first = RemoteGateway::start(&data_root, owner_tx.clone()).unwrap();
    let first_port = endpoint_port(&first.shared.lock().unwrap().endpoint).unwrap();
    let remote_root = data_root.join("v1").join("remote");
    let cert = fs::read(remote_root.join("tls-cert.der")).unwrap();
    let expected_fingerprint = first.shared.lock().unwrap().fingerprint.clone();
    assert_eq!(
        expected_fingerprint,
        Sha256::digest(cert)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&remote_root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(remote_root.join("tls-key.der"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    drop(first);
    let second = RemoteGateway::start(&data_root, owner_tx).unwrap();
    let second_port = endpoint_port(&second.shared.lock().unwrap().endpoint).unwrap();
    assert_eq!(first_port, second_port);
    drop(second);
    cleanup(&data_root);
}

pub(super) fn test_root(name: &str) -> PathBuf {
    let root = crate::test_support::temp_path("pad-remote", name);
    crate::paths::base::ensure_private_dir(&root).unwrap();
    root
}

pub(super) fn test_state(root: &Path) -> Arc<Mutex<GatewayState>> {
    Arc::new(Mutex::new(GatewayState {
        root: root.to_path_buf(),
        endpoint: "wss://pad.local:43827".to_string(),
        fingerprint: "00".repeat(32),
        display_name: "Test Mac".to_string(),
        disk: DiskState {
            enabled: true,
            ..DiskState::default()
        },
        pairing: None,
        last_error: None,
        broker: EventBroker {
            epoch: "epoch-1".to_string(),
            revision: 0,
            ring_bytes: 0,
            ring: VecDeque::new(),
            clients: Vec::new(),
        },
        ephemeral_receipts: VecDeque::new(),
        inflight: HashMap::new(),
    }))
}

fn ticket(id: &str, secret: &str) -> PairingTicket {
    PairingTicket {
        id: id.to_string(),
        secret_hash: digest_secret(secret.as_bytes()),
        profile_id: "a".to_string(),
        expires_at: now().saturating_add(60),
        attempts: 0,
    }
}

fn device() -> DeviceHello {
    DeviceHello {
        display_name: "Tim's iPhone".to_string(),
        platform: "ios".to_string(),
    }
}

pub(super) fn stored_device(id: &str, profile_id: &str, token: &str) -> StoredDevice {
    StoredDevice {
        id: id.to_string(),
        display_name: "Phone".to_string(),
        platform: "ios".to_string(),
        profile_id: profile_id.to_string(),
        token_hash: token_hash(id, token),
        paired_at: now(),
        last_seen_at: None,
        revoked: false,
    }
}

pub(super) fn test_client(
    profile_id: &str,
    sender: tokio::sync::mpsc::Sender<QueuedFrame>,
) -> ClientSink {
    let (close, _receiver) = tokio::sync::watch::channel(None);
    let (overflow_notify, _overflow_receiver) = tokio::sync::watch::channel(false);
    ClientSink {
        connection_id: format!("connection-{profile_id}"),
        device_id: format!("device-{profile_id}"),
        profile_id: profile_id.to_string(),
        sender,
        queued_bytes: Arc::new(AtomicUsize::new(0)),
        overflowed: Arc::new(AtomicBool::new(false)),
        overflow_notify,
        close,
        _acknowledged: Arc::new(AtomicU64::new(0)),
    }
}

pub(super) fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
}
