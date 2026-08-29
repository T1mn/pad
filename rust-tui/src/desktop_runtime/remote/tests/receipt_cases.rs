use super::super::network::execute_command;
use super::super::*;
use super::{cleanup, stored_device, test_root, test_state};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn command_id_is_bound_to_action_and_canonical_params() {
    let root = test_root("receipt-request-binding");
    let shared = test_state(&root);
    shared
        .lock()
        .unwrap()
        .disk
        .devices
        .push(stored_device("device-a", "a", "token"));
    let (owner_tx, owner_rx) = std::sync::mpsc::sync_channel(4);
    let owner = std::thread::spawn(move || {
        let ServerInput::Remote(request) = owner_rx.recv_timeout(Duration::from_secs(2)).unwrap()
        else {
            panic!("expected remote command")
        };
        request
            .response
            .send(RemoteCommandOutcome {
                ok: true,
                result: Some(json!({"accepted":true})),
                error: None,
            })
            .unwrap();
        assert!(owner_rx.recv_timeout(Duration::from_millis(150)).is_err());
    });
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let first = runtime.block_on(execute_command(
        &shared,
        &owner_tx,
        "device-a",
        "stable-id",
        "prompt",
        json!({"task_id":"a-task","prompt":"hello"}),
    ));
    assert!(first.ok);
    let replay = runtime.block_on(execute_command(
        &shared,
        &owner_tx,
        "device-a",
        "stable-id",
        "prompt",
        json!({"prompt":"hello","task_id":"a-task"}),
    ));
    assert_eq!(first, replay, "object key order must not change the digest");
    for (action, params) in [
        ("prompt", json!({"task_id":"a-task","prompt":"different"})),
        ("abort", json!({"task_id":"a-task"})),
    ] {
        let conflict = runtime.block_on(execute_command(
            &shared,
            &owner_tx,
            "device-a",
            "stable-id",
            action,
            params,
        ));
        assert_eq!(conflict.error.unwrap().code, "command_id_conflict");
    }
    owner.join().unwrap();
    assert_eq!(shared.lock().unwrap().disk.receipts.len(), 1);
    cleanup(&root);
}

pub(crate) fn concurrent_command_id_conflict_executes_only_one_payload() {
    let root = test_root("receipt-concurrent-conflict");
    let shared = test_state(&root);
    shared
        .lock()
        .unwrap()
        .disk
        .devices
        .push(stored_device("device-a", "a", "token"));
    let (owner_tx, owner_rx) = std::sync::mpsc::sync_channel(4);
    let executions = Arc::new(AtomicU64::new(0));
    let owner_executions = Arc::clone(&executions);
    let owner = std::thread::spawn(move || {
        let ServerInput::Remote(request) = owner_rx.recv_timeout(Duration::from_secs(2)).unwrap()
        else {
            panic!("expected remote command")
        };
        owner_executions.fetch_add(1, Ordering::AcqRel);
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
                "racing-id",
                "prompt",
                json!({"task_id":"a-task","prompt":"left"}),
            ),
            execute_command(
                &shared,
                &owner_tx,
                "device-a",
                "racing-id",
                "prompt",
                json!({"task_id":"a-task","prompt":"right"}),
            )
        )
    });
    owner.join().unwrap();
    assert_eq!(executions.load(Ordering::Acquire), 1);
    assert_eq!(usize::from(left.ok) + usize::from(right.ok), 1);
    let conflict = if left.ok { right } else { left };
    assert_eq!(conflict.error.unwrap().code, "command_id_conflict");
    cleanup(&root);
}

pub(crate) fn read_receipts_stay_in_memory_and_persistent_receipts_are_bounded() {
    let root = test_root("receipt-bounds");
    let shared = test_state(&root);
    shared
        .lock()
        .unwrap()
        .disk
        .devices
        .push(stored_device("device-a", "a", "token"));
    let (owner_tx, owner_rx) = std::sync::mpsc::sync_channel(2);
    let owner = std::thread::spawn(move || {
        let ServerInput::Remote(request) = owner_rx.recv_timeout(Duration::from_secs(2)).unwrap()
        else {
            panic!("expected remote command")
        };
        request
            .response
            .send(RemoteCommandOutcome {
                ok: true,
                result: Some(json!({"messages":["snapshot"]})),
                error: None,
            })
            .unwrap();
    });
    let outcome = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(execute_command(
            &shared,
            &owner_tx,
            "device-a",
            "read-id",
            "history",
            json!({"task_id":"a-task"}),
        ));
    assert!(outcome.ok);
    owner.join().unwrap();
    let mut state = shared.lock().unwrap();
    assert!(state.disk.receipts.is_empty());
    assert_eq!(state.ephemeral_receipts.len(), 1);
    for index in 0..(RECEIPT_LIMIT + 20) {
        state.disk.receipts.push(StoredReceipt {
            device_id: "device-a".to_string(),
            command_id: format!("bounded-{index}"),
            request_fingerprint: "ab".repeat(32),
            in_progress: false,
            completed_at: now(),
            outcome: RemoteCommandOutcome {
                ok: true,
                result: Some(json!({"value":"x".repeat(20 * 1024)})),
                error: None,
            },
        });
    }
    prune_receipts(&mut state.disk, now());
    assert!(state.disk.receipts.len() <= RECEIPT_LIMIT);
    assert!(receipt_storage_bytes(&state.disk.receipts) <= RECEIPT_BYTES_LIMIT);
    drop(state);
    cleanup(&root);
}

pub(crate) fn restart_with_in_progress_mutation_returns_unknown_without_reexecution() {
    let root = test_root("receipt-restart-marker");
    let params = json!({"task_id":"a-task","prompt":"once"});
    let mut disk = DiskState {
        enabled: true,
        ..DiskState::default()
    };
    disk.devices.push(stored_device("device-a", "a", "token"));
    disk.receipts.push(StoredReceipt {
        device_id: "device-a".to_string(),
        command_id: "crashed-id".to_string(),
        request_fingerprint: super::super::receipts::request_fingerprint("prompt", &params),
        in_progress: true,
        completed_at: now(),
        outcome: RemoteCommandOutcome::rejected(
            "command_outcome_unknown",
            "command outcome is unknown; resync before issuing a new command",
        ),
    });
    persist_disk_state(&root, &disk).unwrap();
    let restarted = test_state(&root);
    restarted.lock().unwrap().disk = load_disk_state(&root).unwrap();
    let (owner_tx, owner_rx) = std::sync::mpsc::sync_channel(1);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let outcome = runtime.block_on(execute_command(
        &restarted,
        &owner_tx,
        "device-a",
        "crashed-id",
        "prompt",
        params,
    ));
    assert_eq!(outcome.error.unwrap().code, "command_outcome_unknown");
    assert!(
        owner_rx.try_recv().is_err(),
        "crashed mutation was re-executed"
    );
    let conflict = runtime.block_on(execute_command(
        &restarted,
        &owner_tx,
        "device-a",
        "crashed-id",
        "prompt",
        json!({"task_id":"a-task","prompt":"different"}),
    ));
    assert_eq!(conflict.error.unwrap().code, "command_id_conflict");
    cleanup(&root);
}

pub(crate) fn mutation_receipt_persistence_failure_is_never_reported_as_success() {
    let root = test_root("receipt-persist-failure");
    let shared = test_state(&root);
    {
        let mut state = shared.lock().unwrap();
        state
            .disk
            .devices
            .push(stored_device("device-a", "a", "token"));
        state.root = root.join("missing-parent").join("remote");
    }
    let (owner_tx, owner_rx) = std::sync::mpsc::sync_channel(2);
    let outcome = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(execute_command(
            &shared,
            &owner_tx,
            "device-a",
            "mutation-id",
            "prompt",
            json!({"task_id":"a-task","prompt":"hello"}),
        ));
    assert!(!outcome.ok);
    assert_eq!(outcome.error.unwrap().code, "receipt_persistence_failed");
    assert!(
        owner_rx.try_recv().is_err(),
        "mutation ran without a durable marker"
    );
    let state = shared.lock().unwrap();
    assert!(state.disk.receipts.is_empty());
    cleanup(&root);
}
