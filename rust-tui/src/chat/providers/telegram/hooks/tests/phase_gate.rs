use super::support::{pending_request, stop_event};

#[test]
fn stop_is_ignored_while_pending_still_awaits_submit() {
    let pending = pending_request(None, "awaiting_submit", None, 0);
    let event = stop_event(Some("turn-old"), "old answer");

    assert!(!pending_can_complete_from_stop(&pending, &event));
}
