use super::*;

mod completion;
mod direct;
mod journal;
mod pending_match;

#[cfg(test)]
use completion::resolve_pending_result_text;
pub(super) use direct::{daemon_socket_is_active, start_direct_hook_listener};
#[cfg(test)]
pub(super) use journal::should_probe_hook_journal_inner;
pub(super) use journal::{
    remember_processed_hook_event, should_probe_hook_journal, sync_state_from_disk_public,
};
pub(super) use pending_match::apply_hook_event_to_pending;
#[cfg(test)]
pub(super) use pending_match::{
    hook_event_matches_pending_turn, matching_pending_request_index, pending_can_complete_from_stop,
};

#[cfg(test)]
mod tests;
