use crate::app::PreviewDetailCache;
use crate::model::SharedPreviewTurns;

pub(super) fn detail_cache_matches_current_turns(
    cache: &PreviewDetailCache,
    turns: &SharedPreviewTurns,
    target_key: &str,
    turn_index: usize,
    width: u16,
    theme_name: &str,
) -> bool {
    cache.target_key == target_key
        && cache.turn_index == turn_index
        && cache.width == width
        && cache.theme_name == theme_name
        && cache.turns.shares_allocation_with(turns)
}

pub(super) fn detail_cache_matches_request(
    cache: &PreviewDetailCache,
    target_key: &str,
    turn_index: usize,
    width: u16,
    theme_name: &str,
    question: &str,
    answer: &Option<String>,
) -> bool {
    cache.target_key == target_key
        && cache.turn_index == turn_index
        && cache.width == width
        && cache.theme_name == theme_name
        && cache.question == question
        && cache.answer == *answer
}
