use super::direct::daemon_socket_is_active;
use super::*;

fn sync_state_from_disk(state: &mut TelegramState) {
    if let Ok(mut latest) = load_state() {
        latest.journal_position = latest.journal_position.max(state.journal_position);
        latest.last_journal_recovery_at = latest
            .last_journal_recovery_at
            .max(state.last_journal_recovery_at);
        *state = latest;
    }
}

pub(in crate::chat::providers::telegram) fn sync_state_from_disk_public(state: &mut TelegramState) {
    sync_state_from_disk(state);
}

pub(in crate::chat::providers::telegram) fn should_probe_hook_journal(
    state: &TelegramState,
) -> bool {
    should_probe_hook_journal_inner(state, daemon_socket_is_active(), now_ts())
}

pub(in crate::chat::providers::telegram) fn should_probe_hook_journal_inner(
    state: &TelegramState,
    direct_hook_active: bool,
    now: i64,
) -> bool {
    if state.pending_requests.is_empty() {
        return false;
    }
    if state.last_journal_recovery_at == 0 {
        return true;
    }
    if !direct_hook_active {
        return now.saturating_sub(state.last_journal_recovery_at) >= 1;
    }
    if now.saturating_sub(state.last_journal_recovery_at) < JOURNAL_RECOVERY_RETRY_SECS {
        return false;
    }
    state
        .pending_requests
        .iter()
        .any(|pending| match pending.phase.as_str() {
            "awaiting_submit" => now.saturating_sub(pending.sent_at) >= JOURNAL_RECOVERY_STALL_SECS,
            "awaiting_stop" | "awaiting_confirm" => {
                now.saturating_sub(pending.accepted_at.unwrap_or(pending.sent_at))
                    >= JOURNAL_RECOVERY_STALL_SECS
            }
            _ => false,
        })
}

pub(in crate::chat::providers::telegram) fn remember_processed_hook_event(
    state: &mut TelegramState,
    event: &HookEvent,
) -> bool {
    let signature = hook_event_signature(event);
    if recent_hook_signature_exists(&signature) {
        return false;
    }
    if state
        .processed_hook_signatures
        .iter()
        .any(|existing| existing == &signature)
    {
        return false;
    }
    state.processed_hook_signatures.push(signature);
    const MAX_PROCESSED_HOOKS: usize = 64;
    if state.processed_hook_signatures.len() > MAX_PROCESSED_HOOKS {
        let drop_count = state.processed_hook_signatures.len() - MAX_PROCESSED_HOOKS;
        state.processed_hook_signatures.drain(0..drop_count);
    }
    remember_recent_hook_signature(
        state
            .processed_hook_signatures
            .last()
            .expect("processed hook signature must exist"),
    );
    true
}

fn hook_event_signature(event: &HookEvent) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        event.event,
        event.tmux.pane_id.as_deref().unwrap_or(""),
        event.turn_id.as_deref().unwrap_or(""),
        event.timestamp.as_deref().unwrap_or(""),
        event.session_id.as_deref().unwrap_or(""),
        event
            .prompt
            .as_deref()
            .map(|prompt| format!("{:x}", md5::compute(prompt.as_bytes())))
            .unwrap_or_default()
    )
}

fn recent_hook_signature_exists(signature: &str) -> bool {
    RECENT_HOOK_SIGNATURES
        .lock()
        .map(|signatures| signatures.iter().any(|existing| existing == signature))
        .unwrap_or(false)
}

fn remember_recent_hook_signature(signature: &str) {
    if let Ok(mut signatures) = RECENT_HOOK_SIGNATURES.lock() {
        signatures.push(signature.to_string());
        const MAX_RECENT_HOOK_SIGNATURES: usize = 128;
        if signatures.len() > MAX_RECENT_HOOK_SIGNATURES {
            let drop_count = signatures.len() - MAX_RECENT_HOOK_SIGNATURES;
            signatures.drain(0..drop_count);
        }
    }
}
