mod advance;
mod apply;
mod matching;

pub(super) use advance::advance_pending_to_awaiting_stop;
pub(in crate::chat::providers::telegram) use apply::apply_hook_event_to_pending;
pub(in crate::chat::providers::telegram) use matching::pending_can_complete_from_stop;
pub(super) use matching::pending_matches_submit_prompt;
#[cfg(test)]
pub(in crate::chat::providers::telegram) use matching::{
    hook_event_matches_pending_turn, matching_pending_request_index,
};
