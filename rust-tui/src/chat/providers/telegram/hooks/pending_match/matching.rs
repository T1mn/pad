use super::super::*;

pub(in crate::chat::providers::telegram) fn matching_pending_request_index(
    state: &TelegramState,
    event: &HookEvent,
) -> Option<usize> {
    let pane_id = event.tmux.pane_id.as_deref()?;
    let pending_index = pending_request_index_by_pane(state, pane_id)?;
    let pending = state.pending_requests.get(pending_index)?;
    match event.event.as_str() {
        "user_prompt_submit" if pending_matches_submit_prompt(pending, event) => {
            Some(pending_index)
        }
        "stop" if pending_can_complete_from_stop(pending, event) => Some(pending_index),
        _ => None,
    }
}

pub(in crate::chat::providers::telegram) fn pending_can_complete_from_stop(
    pending: &PendingRequest,
    event: &HookEvent,
) -> bool {
    matches!(pending.phase.as_str(), "awaiting_stop" | "awaiting_confirm")
        && hook_event_matches_pending_turn(pending, event)
}

pub(in crate::chat::providers::telegram) fn hook_event_matches_pending_turn(
    pending: &PendingRequest,
    event: &HookEvent,
) -> bool {
    match (pending.turn_id.as_deref(), event.turn_id.as_deref()) {
        (Some(_), None) if pending.agent_kind == "codex" => false,
        (Some(expected), Some(actual)) => expected == actual,
        _ => true,
    }
}

pub(in crate::chat::providers::telegram::hooks) fn pending_matches_submit_prompt(
    pending: &PendingRequest,
    event: &HookEvent,
) -> bool {
    event
        .prompt
        .as_deref()
        .map(|prompt| format!("{:x}", md5::compute(prompt.as_bytes())) == pending.prompt_hash)
        .unwrap_or(true)
}
