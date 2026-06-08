use super::super::*;
use std::path::Path;

pub(super) struct ApprovalScanOutcome {
    pub(super) changed: bool,
    pub(super) previous_call_id: Option<String>,
    pub(super) next_request: Option<CodexApprovalRequest>,
}

pub(super) fn scan_and_apply_approval_state(
    state: &mut TelegramState,
    request_id: &str,
    snapshot: &PendingRequest,
    transcript_path: &str,
) -> TelegramResult<ApprovalScanOutcome> {
    let previous_call_id = snapshot.approval_call_id.clone();
    let scan_result = scan_codex_approval_updates(
        Path::new(transcript_path),
        snapshot.approval_scan_offset,
        current_approval_request(snapshot),
    )?;

    let next_request = scan_result.active_request.clone();
    let changed = previous_call_id.as_deref()
        != next_request
            .as_ref()
            .map(|request| request.call_id.as_str());

    apply_approval_scan_result(
        state,
        request_id,
        scan_result.next_offset,
        next_request.as_ref(),
    );

    Ok(ApprovalScanOutcome {
        changed,
        previous_call_id,
        next_request,
    })
}

fn current_approval_request(snapshot: &PendingRequest) -> Option<CodexApprovalRequest> {
    snapshot
        .approval_call_id
        .clone()
        .zip(snapshot.approval_justification.clone())
        .map(|(call_id, justification)| CodexApprovalRequest {
            call_id,
            justification,
        })
}

fn apply_approval_scan_result(
    state: &mut TelegramState,
    request_id: &str,
    next_offset: u64,
    next_request: Option<&CodexApprovalRequest>,
) {
    let Some(index) = pending_request_index_by_id(state, request_id) else {
        return;
    };
    let pending = &mut state.pending_requests[index];
    pending.approval_scan_offset = next_offset;
    match next_request {
        Some(request) => {
            pending.phase = "awaiting_confirm".to_string();
            pending.approval_call_id = Some(request.call_id.clone());
            pending.approval_justification = Some(request.justification.clone());
            pending.last_status_at = None;
        }
        None => {
            pending.approval_call_id = None;
            pending.approval_justification = None;
            if pending.phase == "awaiting_confirm" {
                pending.phase = "awaiting_stop".to_string();
            }
            pending.last_status_at = None;
        }
    }
}
