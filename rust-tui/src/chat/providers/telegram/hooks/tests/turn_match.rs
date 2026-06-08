use super::support::{pending_request, stop_event};

#[test]
fn pending_turn_must_match_stop_turn_when_both_exist() {
    let pending = pending_request(Some("turn-a"), "awaiting_stop", None, 0);
    let mut event = stop_event(Some("turn-b"), "wrong turn");
    event.timestamp = Some("2026-04-07T00:00:00Z".into());

    assert!(!hook_event_matches_pending_turn(&pending, &event));
}

#[test]
fn codex_stop_without_turn_id_is_ignored_when_pending_turn_exists() {
    let pending = pending_request(Some("turn-a"), "awaiting_stop", None, 0);
    let event = stop_event(None, "missing turn");

    assert!(!hook_event_matches_pending_turn(&pending, &event));
}
