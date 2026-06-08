use super::super::*;

pub(super) fn ensure_approval_transcript_path(
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
        if pending.approval_scan_offset == 0 {
            pending.approval_scan_offset = transcript_len(&path).saturating_sub(32 * 1024);
        }
    }

    Ok(Some(path))
}
