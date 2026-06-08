use super::super::*;

pub(super) fn approval_request_ids(state: &TelegramState) -> Vec<String> {
    state
        .pending_requests
        .iter()
        .filter(|pending| approval_scan_candidate(pending))
        .map(|pending| pending.request_id.clone())
        .collect()
}

pub(super) fn approval_snapshot(state: &TelegramState, request_id: &str) -> Option<PendingRequest> {
    let snapshot = state
        .pending_requests
        .iter()
        .find(|pending| pending.request_id == request_id)
        .cloned()?;
    approval_scan_candidate(&snapshot).then_some(snapshot)
}

fn approval_scan_candidate(pending: &PendingRequest) -> bool {
    pending.agent_kind == "codex"
        && (pending.accepted_at.is_some() || pending.phase == "awaiting_confirm")
}
