use super::*;

#[test]
fn journal_recovery_runs_immediately_for_pending_on_startup() {
    let state = TelegramState {
        pending_requests: vec![sample_pending("tg-1", "%1", "awaiting_submit")],
        ..TelegramState::default()
    };

    assert!(should_probe_hook_journal_inner(&state, true, 100));
}
#[test]
fn journal_recovery_waits_for_stall_when_direct_hook_is_alive() {
    let state = TelegramState {
        last_journal_recovery_at: 100,
        pending_requests: vec![PendingRequest {
            sent_at: 101,
            sent_at_ms: 101_000,
            ..sample_pending("tg-1", "%1", "awaiting_submit")
        }],
        ..TelegramState::default()
    };

    assert!(!should_probe_hook_journal_inner(&state, true, 103));
    assert!(should_probe_hook_journal_inner(&state, true, 106));
}
#[test]
fn journal_recovery_probes_if_any_pending_request_is_stalled() {
    let state = TelegramState {
        last_journal_recovery_at: 100,
        pending_requests: vec![
            PendingRequest {
                sent_at: 103,
                sent_at_ms: 103_000,
                ..sample_pending("tg-1", "%1", "awaiting_submit")
            },
            PendingRequest {
                accepted_at: Some(101),
                accepted_at_ms: Some(101_000),
                turn_id: Some("turn-2".into()),
                ..sample_pending("tg-2", "%2", "awaiting_stop")
            },
        ],
        ..TelegramState::default()
    };

    assert!(should_probe_hook_journal_inner(&state, true, 106));
}
