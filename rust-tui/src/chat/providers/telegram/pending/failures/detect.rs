use super::super::*;

#[derive(Clone, Debug)]
pub(in crate::chat::providers::telegram::pending) struct PendingRolloutFailureResolution {
    pub(in crate::chat::providers::telegram::pending) pending: PendingRequest,
    pub(in crate::chat::providers::telegram::pending) failure:
        crate::chat::approval::CodexFailureEvent,
    pub(in crate::chat::providers::telegram::pending) continuity:
        Option<crate::session_continuity::ContinuitySnapshot>,
}

pub(in crate::chat::providers::telegram::pending) fn pending_rollout_failure_check_due(
    pending: &PendingRequest,
    now: i64,
) -> bool {
    if pending.agent_kind != "codex" {
        return false;
    }
    if !matches!(pending.phase.as_str(), "awaiting_stop" | "awaiting_confirm") {
        return false;
    }
    let Some(accepted_at) = pending.accepted_at else {
        return false;
    };
    if now.saturating_sub(accepted_at) < PENDING_FAILURE_SCAN_DELAY_SECS {
        return false;
    }
    pending
        .last_failure_check_at
        .map(|last_checked| now.saturating_sub(last_checked) >= PENDING_FAILURE_SCAN_INTERVAL_SECS)
        .unwrap_or(true)
}

pub(in crate::chat::providers::telegram::pending) fn detect_pending_rollout_failure_for_request(
    state: &mut TelegramState,
    request_id: &str,
    checked_at: i64,
) -> TelegramResult<Option<PendingRolloutFailureResolution>> {
    let Some(snapshot) = state
        .pending_requests
        .iter()
        .find(|pending| pending.request_id == request_id)
        .cloned()
    else {
        return Ok(None);
    };
    if !pending_rollout_failure_check_due(&snapshot, checked_at) {
        return Ok(None);
    }

    if let Some(index) = pending_request_index_by_id(state, request_id) {
        state.pending_requests[index].last_failure_check_at = Some(checked_at);
    }

    let Some(transcript_path) = ensure_pending_transcript_path(state, request_id, &snapshot)?
    else {
        return Ok(None);
    };
    let scan_result = crate::chat::approval::scan_codex_failure_updates(
        Path::new(&transcript_path),
        snapshot.failure_scan_offset,
        snapshot.turn_id.as_deref(),
    )?;

    if let Some(index) = pending_request_index_by_id(state, request_id) {
        let pending = &mut state.pending_requests[index];
        pending.failure_scan_offset = scan_result.next_offset;
    }

    let Some(failure) = scan_result.failure else {
        return Ok(None);
    };
    let continuity = crate::session_continuity::load_snapshot_for(
        snapshot.session_id.as_deref(),
        Some(&transcript_path),
    );
    let pending = remove_pending_request(state, request_id).unwrap_or(snapshot);
    Ok(Some(PendingRolloutFailureResolution {
        pending,
        failure,
        continuity,
    }))
}

fn ensure_pending_transcript_path(
    state: &mut TelegramState,
    request_id: &str,
    snapshot: &PendingRequest,
) -> TelegramResult<Option<String>> {
    if let Some(path) = snapshot.transcript_path.clone() {
        return Ok(Some(path));
    }

    let path = live_panels()
        .map_err(telegram_error)?
        .into_iter()
        .find(|panel| panel.pane_id == snapshot.pane_id)
        .and_then(|panel| panel.transcript_path);
    let Some(path) = path else {
        return Ok(None);
    };

    if let Some(index) = pending_request_index_by_id(state, request_id) {
        let pending = &mut state.pending_requests[index];
        pending.transcript_path = Some(path.clone());
        if pending.failure_scan_offset == 0 {
            pending.failure_scan_offset = if pending.result_scan_offset > 0 {
                pending.result_scan_offset
            } else {
                transcript_len(&path)
            };
        }
    }

    Ok(Some(path))
}
