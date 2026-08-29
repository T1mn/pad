use super::receipts::{bounded_command_outcome, command_id_conflict, request_fingerprint};
use super::{
    now, persist_disk_state, prune_receipts, remote_action_mutates, ConnectionCloseReason,
    GatewayState, InflightCommand, RemoteCommandOutcome, StoredReceipt, RECEIPT_BYTES_LIMIT,
    RECEIPT_LIMIT, RECEIPT_TTL_SECONDS,
};
use super::{RemoteOwnerRequest, ServerInput};
use serde_json::Value;
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(super) async fn execute_command(
    shared: &Arc<Mutex<GatewayState>>,
    owner_tx: &SyncSender<ServerInput>,
    device_id: &str,
    command_id: &str,
    action: &str,
    params: Value,
) -> RemoteCommandOutcome {
    if command_id.is_empty() || command_id.len() > 256 {
        return RemoteCommandOutcome::rejected("invalid_command_id", "command_id is invalid");
    }
    let fingerprint = request_fingerprint(action, &params);
    let inflight_key = format!("{device_id}\0{command_id}");
    let waiter = {
        let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
        if !state.disk.enabled {
            return RemoteCommandOutcome::rejected("remote_disabled", "remote access is disabled");
        }
        if !state
            .disk
            .devices
            .iter()
            .any(|device| device.id == device_id && !device.revoked)
        {
            return RemoteCommandOutcome::rejected("device_revoked", "remote device was revoked");
        }
        prune_receipts(&mut state.disk, now());
        prune_ephemeral_receipts(&mut state, now());
        let receipt = state
            .disk
            .receipts
            .iter()
            .chain(state.ephemeral_receipts.iter())
            .find(|receipt| receipt.device_id == device_id && receipt.command_id == command_id)
            .map(|receipt| {
                (
                    receipt.request_fingerprint.clone(),
                    receipt.in_progress,
                    receipt.outcome.clone(),
                )
            });
        if let Some((stored_fingerprint, in_progress, outcome)) = receipt {
            if stored_fingerprint != fingerprint {
                return command_id_conflict();
            }
            if !in_progress {
                return outcome;
            }
            if !state.inflight.contains_key(&inflight_key) {
                return unknown_outcome();
            }
        }
        if let Some(inflight) = state.inflight.get_mut(&inflight_key) {
            if inflight.request_fingerprint != fingerprint {
                return command_id_conflict();
            }
            let (sender, receiver) = mpsc::channel();
            inflight.waiters.push(sender);
            Some(receiver)
        } else {
            state.inflight.insert(
                inflight_key.clone(),
                InflightCommand {
                    request_fingerprint: fingerprint.clone(),
                    waiters: Vec::new(),
                },
            );
            None
        }
    };
    if let Some(waiter) = waiter {
        return tokio::task::spawn_blocking(move || waiter.recv_timeout(Duration::from_secs(95)))
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or_else(unknown_outcome);
    }
    if remote_action_mutates(action)
        && persist_in_progress_marker(shared, device_id, command_id, &fingerprint, &inflight_key)
            .is_err()
    {
        let error = RemoteCommandOutcome::rejected(
            "receipt_persistence_failed",
            "command was not executed because its durable receipt could not be prepared",
        );
        notify_waiters(shared, &inflight_key, &error);
        return error;
    }
    let (response_tx, response_rx) = mpsc::channel();
    let request = RemoteOwnerRequest {
        device_id: device_id.to_string(),
        action: action.to_string(),
        params,
        response: response_tx,
    };
    let delivery_error = match owner_tx.try_send(ServerInput::Remote(request)) {
        Ok(()) => None,
        Err(TrySendError::Full(_)) => Some(RemoteCommandOutcome::rejected(
            "server_busy",
            "Desktop owner queue is full",
        )),
        Err(TrySendError::Disconnected(_)) => Some(RemoteCommandOutcome::rejected(
            "server_unavailable",
            "Desktop owner is unavailable",
        )),
    };
    if let Some(error) = delivery_error {
        let error = if remote_action_mutates(action)
            && !remove_unstarted_marker(shared, device_id, command_id)
        {
            unknown_outcome()
        } else {
            error
        };
        notify_waiters(shared, &inflight_key, &error);
        return error;
    }
    let outcome =
        tokio::task::spawn_blocking(move || response_rx.recv_timeout(Duration::from_secs(90)))
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or_else(unknown_outcome);
    finish_command(
        shared,
        inflight_key,
        device_id,
        command_id,
        action,
        fingerprint,
        bounded_command_outcome(command_id, outcome),
    )
}

fn finish_command(
    shared: &Arc<Mutex<GatewayState>>,
    inflight_key: String,
    device_id: &str,
    command_id: &str,
    action: &str,
    fingerprint: String,
    outcome: RemoteCommandOutcome,
) -> RemoteCommandOutcome {
    let (waiters, outcome) = {
        let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
        prune_receipts(&mut state.disk, now());
        let mut receipt = StoredReceipt {
            device_id: device_id.to_string(),
            command_id: command_id.to_string(),
            request_fingerprint: fingerprint,
            in_progress: false,
            completed_at: now(),
            outcome: outcome.clone(),
        };
        let final_outcome = if remote_action_mutates(action) {
            persist_mutation_receipt(&mut state, &mut receipt, outcome)
        } else {
            push_ephemeral_receipt(&mut state, receipt);
            outcome
        };
        let waiters = state
            .inflight
            .remove(&inflight_key)
            .map(|inflight| inflight.waiters)
            .unwrap_or_default();
        (waiters, final_outcome)
    };
    for waiter in waiters {
        let _ = waiter.send(outcome.clone());
    }
    outcome
}

fn persist_mutation_receipt(
    state: &mut GatewayState,
    receipt: &mut StoredReceipt,
    outcome: RemoteCommandOutcome,
) -> RemoteCommandOutcome {
    let previous = state.disk.receipts.clone();
    if let Some(stored) = state.disk.receipts.iter_mut().find(|stored| {
        stored.device_id == receipt.device_id && stored.command_id == receipt.command_id
    }) {
        *stored = receipt.clone();
    } else {
        state.disk.receipts.push(receipt.clone());
    }
    prune_receipts(&mut state.disk, now());
    if persist_disk_state(&state.root, &state.disk).is_ok() {
        return outcome;
    }
    state.disk.receipts = previous;
    state.disk.enabled = false;
    state.last_error = Some("receipt_persistence_failed".to_string());
    let unknown = RemoteCommandOutcome::rejected(
        "command_outcome_unknown",
        "command completed but its receipt could not be saved; resync before issuing a new command",
    );
    receipt.outcome = unknown.clone();
    push_ephemeral_receipt(state, receipt.clone());
    for client in &state.broker.clients {
        let _ = client
            .close
            .send(Some(ConnectionCloseReason::RemoteDisabled));
    }
    state.broker.clients.clear();
    unknown
}

fn persist_in_progress_marker(
    shared: &Arc<Mutex<GatewayState>>,
    device_id: &str,
    command_id: &str,
    fingerprint: &str,
    inflight_key: &str,
) -> Result<(), ()> {
    let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
    let previous = state.disk.receipts.clone();
    state.disk.receipts.push(StoredReceipt {
        device_id: device_id.to_string(),
        command_id: command_id.to_string(),
        request_fingerprint: fingerprint.to_string(),
        in_progress: true,
        completed_at: now(),
        outcome: unknown_outcome(),
    });
    prune_receipts(&mut state.disk, now());
    if persist_disk_state(&state.root, &state.disk).is_ok() {
        return Ok(());
    }
    state.disk.receipts = previous;
    state.inflight.remove(inflight_key);
    Err(())
}

fn remove_unstarted_marker(
    shared: &Arc<Mutex<GatewayState>>,
    device_id: &str,
    command_id: &str,
) -> bool {
    let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
    let previous = state.disk.receipts.clone();
    state.disk.receipts.retain(|receipt| {
        !(receipt.device_id == device_id && receipt.command_id == command_id && receipt.in_progress)
    });
    if persist_disk_state(&state.root, &state.disk).is_ok() {
        true
    } else {
        state.disk.receipts = previous;
        false
    }
}

fn notify_waiters(
    shared: &Arc<Mutex<GatewayState>>,
    inflight_key: &str,
    outcome: &RemoteCommandOutcome,
) {
    let waiters = shared
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .inflight
        .remove(inflight_key)
        .map(|inflight| inflight.waiters)
        .unwrap_or_default();
    for waiter in waiters {
        let _ = waiter.send(outcome.clone());
    }
}

fn unknown_outcome() -> RemoteCommandOutcome {
    RemoteCommandOutcome::rejected(
        "command_outcome_unknown",
        "command outcome is unknown; resync before issuing a new command",
    )
}

fn prune_ephemeral_receipts(state: &mut GatewayState, timestamp: u64) {
    state
        .ephemeral_receipts
        .retain(|receipt| timestamp.saturating_sub(receipt.completed_at) <= RECEIPT_TTL_SECONDS);
    while state.ephemeral_receipts.len() > RECEIPT_LIMIT
        || ephemeral_receipt_bytes(state) > RECEIPT_BYTES_LIMIT
    {
        state.ephemeral_receipts.pop_front();
    }
}

fn push_ephemeral_receipt(state: &mut GatewayState, receipt: StoredReceipt) {
    state.ephemeral_receipts.push_back(receipt);
    prune_ephemeral_receipts(state, now());
}

fn ephemeral_receipt_bytes(state: &GatewayState) -> usize {
    state.ephemeral_receipts.iter().fold(0, |total, receipt| {
        total.saturating_add(
            serde_json::to_vec(receipt)
                .map_or(RECEIPT_BYTES_LIMIT.saturating_add(1), |value| value.len()),
        )
    })
}
