use crate::ui::preview::session::{SESSION_ITEM_CONTENT_HEIGHT, SESSION_ITEM_GAP_HEIGHT};

pub(crate) fn selected_session_list_range(
    selected_turn: Option<usize>,
    turn_count: usize,
) -> Option<(usize, usize)> {
    let selected = selected_turn.filter(|index| *index < turn_count)?;
    let stride = SESSION_ITEM_CONTENT_HEIGHT + SESSION_ITEM_GAP_HEIGHT;
    let start = selected * stride;
    Some((start, start + SESSION_ITEM_CONTENT_HEIGHT - 1))
}
