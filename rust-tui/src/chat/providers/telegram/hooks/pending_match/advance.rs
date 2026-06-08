use super::super::*;

pub(in crate::chat::providers::telegram::hooks) fn advance_pending_to_awaiting_stop(
    pending: Option<&mut PendingRequest>,
    event: &HookEvent,
    record_accepted_at_ms: bool,
) {
    let Some(pending) = pending else {
        return;
    };
    pending.phase = "awaiting_stop".to_string();
    pending.accepted_at = Some(now_ts());
    if record_accepted_at_ms {
        pending.accepted_at_ms = Some(now_ms_i64());
    }
    if event.turn_id.is_some() {
        pending.turn_id = event.turn_id.clone();
    }
    if event.session_id.is_some() {
        pending.session_id = event.session_id.clone();
    }
    if event.cwd.is_some() {
        pending.working_dir = event.cwd.clone().unwrap_or_default();
    }
    if event.transcript_path.is_some() {
        pending.transcript_path = event.transcript_path.clone();
    }
    if pending.result_scan_offset == 0 {
        if let Some(path) = pending.transcript_path.as_deref() {
            pending.result_scan_offset = transcript_len(path);
        }
    }
    if pending.failure_scan_offset == 0 {
        if let Some(path) = pending.transcript_path.as_deref() {
            pending.failure_scan_offset = transcript_len(path);
        }
    }
}
