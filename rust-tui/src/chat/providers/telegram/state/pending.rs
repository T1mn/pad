use super::model::{PendingRequest, TelegramState};

pub(in crate::chat::providers::telegram) fn mark_update_processed(
    state: &mut TelegramState,
    update_id: i64,
) -> bool {
    if update_id <= state.last_processed_update_id {
        return false;
    }
    state.last_processed_update_id = update_id;
    state.update_offset = state.update_offset.max(update_id.saturating_add(1));
    true
}

pub(in crate::chat::providers::telegram) fn pending_request_index_by_id(
    state: &TelegramState,
    request_id: &str,
) -> Option<usize> {
    state
        .pending_requests
        .iter()
        .position(|pending| pending.request_id == request_id)
}

pub(in crate::chat::providers::telegram) fn pending_request_index_by_pane(
    state: &TelegramState,
    pane_id: &str,
) -> Option<usize> {
    state
        .pending_requests
        .iter()
        .position(|pending| pending.pane_id == pane_id)
}

pub(in crate::chat::providers::telegram) fn remove_pending_request(
    state: &mut TelegramState,
    request_id: &str,
) -> Option<PendingRequest> {
    let index = pending_request_index_by_id(state, request_id)?;
    Some(state.pending_requests.remove(index))
}

pub(in crate::chat::providers::telegram) fn remove_selected_target_pending_request(
    state: &mut TelegramState,
) -> Option<PendingRequest> {
    let pane_id = state.selected_target.as_ref()?.pane_id.clone();
    let index = pending_request_index_by_pane(state, &pane_id)?;
    Some(state.pending_requests.remove(index))
}
