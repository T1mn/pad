use super::*;

#[test]
fn pending_status_moves_from_accepted_to_working() {
    let mut pending = sample_pending("tg-1", "%1", "awaiting_stop");
    pending.accepted_at = Some(100);
    pending.accepted_at_ms = Some(100_000);

    let accepted = pending_status_text(crate::i18n::Locale::En, &pending, 102);
    let working = pending_status_text(crate::i18n::Locale::En, &pending, 106);

    assert!(accepted.contains("Submitted"));
    assert!(working.contains("Working"));
    assert!(working.contains("6s"));
}
#[test]
fn pending_status_reports_approval_needed() {
    let mut pending = sample_pending("tg-1", "%1", "awaiting_confirm");
    pending.accepted_at = Some(100);
    pending.accepted_at_ms = Some(100_000);
    pending.approval_call_id = Some("call_1".into());
    pending.approval_justification = Some("Do you want to allow running cargo check?".into());

    let text = pending_status_text(crate::i18n::Locale::En, &pending, 110);
    assert!(text.contains("Needs approval"));
    assert!(text.contains("cargo check"));
}
#[test]
fn matching_pending_request_index_targets_correct_submit_event() {
    let mut first = sample_pending("tg-1", "%1", "awaiting_submit");
    first.prompt_hash = format!("{:x}", md5::compute("first".as_bytes()));
    let mut second = sample_pending("tg-2", "%2", "awaiting_submit");
    second.prompt_hash = format!("{:x}", md5::compute("second".as_bytes()));
    let state = TelegramState {
        pending_requests: vec![first, second],
        ..TelegramState::default()
    };
    let event = HookEvent {
        event: "user_prompt_submit".into(),
        turn_id: Some("turn-2".into()),
        session_id: Some("session-2".into()),
        transcript_path: None,
        cwd: Some("/tmp/2".into()),
        prompt: Some("second".into()),
        last_assistant_message: None,
        timestamp: Some("2026-04-14T00:00:00Z".into()),
        tmux: HookTmuxInfo {
            pane_id: Some("%2".into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("2".into()),
            pane_current_path: Some("/tmp/2".into()),
        },
    };

    assert_eq!(matching_pending_request_index(&state, &event), Some(1));
}
#[test]
fn matching_pending_request_index_ignores_stale_stop_in_awaiting_submit() {
    let mut first = sample_pending("tg-1", "%1", "awaiting_submit");
    first.turn_id = Some("turn-1".into());
    let mut second = sample_pending("tg-2", "%2", "awaiting_stop");
    second.turn_id = Some("turn-2".into());
    second.accepted_at = Some(101);
    let state = TelegramState {
        pending_requests: vec![first, second],
        ..TelegramState::default()
    };
    let stale_event = HookEvent {
        event: "stop".into(),
        turn_id: Some("turn-1".into()),
        session_id: Some("session-1".into()),
        transcript_path: None,
        cwd: None,
        prompt: None,
        last_assistant_message: Some("old answer".into()),
        timestamp: Some("2026-04-14T00:00:00Z".into()),
        tmux: HookTmuxInfo {
            pane_id: Some("%1".into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: None,
        },
    };
    let valid_event = HookEvent {
        tmux: HookTmuxInfo {
            pane_id: Some("%2".into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("2".into()),
            pane_current_path: None,
        },
        turn_id: Some("turn-2".into()),
        session_id: Some("session-2".into()),
        ..stale_event.clone()
    };

    assert_eq!(matching_pending_request_index(&state, &stale_event), None);
    assert_eq!(
        matching_pending_request_index(&state, &valid_event),
        Some(1)
    );
}
#[test]
fn approval_lookup_selects_request_by_request_id() {
    let mut first = sample_pending("tg-1", "%1", "awaiting_confirm");
    first.approval_call_id = Some("call-1".into());
    let mut second = sample_pending("tg-2", "%2", "awaiting_confirm");
    second.approval_call_id = Some("call-2".into());
    let state = TelegramState {
        pending_requests: vec![first, second],
        ..TelegramState::default()
    };

    assert_eq!(approval_pending_index(&state, "tg-2"), Some(1));
}
#[test]
fn completed_reply_includes_request_attribution() {
    let mut pending = sample_pending("tg-1", "%9", "delivering_result");
    pending.turn_id = Some("turn-9".into());
    let reply = completed_reply_text(crate::i18n::Locale::En, &pending, "done");

    assert!(reply.contains("Completed"));
    assert!(reply.contains("Request: tg-1"));
    assert!(reply.contains("Target: CODEX"));
    assert!(reply.contains("Pane: %9"));
    assert!(reply.contains("Session: session-9"));
    assert!(reply.contains("Turn: turn-9"));
    assert!(reply.contains("Dir: /tmp/9"));
    assert!(reply.ends_with("done"));
}
#[test]
fn pad_status_lists_all_pending_requests() {
    let state = TelegramState {
        selected_target: Some(SelectedTarget {
            pane_id: "%2".into(),
            label: "CODEX • 2".into(),
        }),
        pending_requests: vec![
            sample_pending("tg-1", "%1", "awaiting_submit"),
            sample_pending("tg-2", "%2", "awaiting_stop"),
        ],
        ..TelegramState::default()
    };

    let body = build_pad_status_body(crate::i18n::Locale::En, "online", &state);
    assert!(body.contains("Pad: online"));
    assert!(body.contains("Target: CODEX • 2"));
    assert!(body.contains("tg-1"));
    assert!(body.contains("tg-2"));
    assert!(body.contains("%1"));
    assert!(body.contains("%2"));
}
#[test]
fn pending_status_summary_line_is_compact_but_identifiable() {
    let pending = sample_pending("tg-1", "%1", "awaiting_submit");
    let summary = pending_status_summary_line(crate::i18n::Locale::En, &pending);
    assert!(summary.contains("tg-1"));
    assert!(summary.contains("%1"));
    assert!(summary.contains("CODEX • 1"));
    assert!(summary.contains("Waiting for submit"));
}
