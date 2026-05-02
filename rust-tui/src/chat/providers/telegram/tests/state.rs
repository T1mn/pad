use super::*;

#[test]
fn processed_updates_are_deduplicated() {
    let mut state = TelegramState::default();
    assert!(mark_update_processed(&mut state, 10));
    assert_eq!(state.last_processed_update_id, 10);
    assert_eq!(state.update_offset, 11);

    assert!(!mark_update_processed(&mut state, 10));
    assert_eq!(state.last_processed_update_id, 10);
    assert_eq!(state.update_offset, 11);

    assert!(!mark_update_processed(&mut state, 9));
    assert_eq!(state.last_processed_update_id, 10);
    assert_eq!(state.update_offset, 11);

    assert!(mark_update_processed(&mut state, 12));
    assert_eq!(state.last_processed_update_id, 12);
    assert_eq!(state.update_offset, 13);
}
#[test]
fn telegram_state_backfills_missing_last_processed_update_id() {
    let state: TelegramState = serde_json::from_str(
        r#"{
            "update_offset": 42,
            "journal_position": 7,
            "agent_snapshot": [],
            "pending": null
        }"#,
    )
    .unwrap();

    assert_eq!(state.update_offset, 42);
    assert_eq!(state.last_processed_update_id, 0);
    assert_eq!(state.journal_position, 7);
    assert!(state.pending_requests.is_empty());
}
#[test]
fn telegram_state_loads_legacy_pending_field_into_pending_requests() {
    with_temp_home("legacy-pending", |_home| {
        let state_path = crate::paths::telegram_state_path();
        std::fs::create_dir_all(state_path.parent().unwrap()).unwrap();
        let body = r#"{
            "update_offset": 42,
            "journal_position": 7,
            "agent_snapshot": [],
            "pending": {
                "request_id": "tg-legacy",
                "chat_id": "1",
                "pane_id": "%7",
                "agent_kind": "codex",
                "target_label": "CODEX • legacy",
                "prompt_text": "hi",
                "prompt_hash": "abc",
                "turn_id": null,
                "sent_at": 100,
                "sent_at_ms": 100000,
                "accepted_at": null,
                "accepted_at_ms": null,
                "last_status_at": null,
                "draft_id": 123,
                "phase": "awaiting_submit",
                "transcript_path": null,
                "result_scan_offset": 0,
                "approval_scan_offset": 0,
                "approval_call_id": null,
                "approval_justification": null,
                "completed_text": null,
                "completed_source": null,
                "delivery_attempts": 0,
                "delivery_retry_at": 0
            }
        }"#;
        fs::write(state_path, body).unwrap();

        let state = load_state().unwrap();
        assert_eq!(state.pending_requests.len(), 1);
        assert_eq!(state.pending_requests[0].request_id, "tg-legacy");
        assert_eq!(state.pending_requests[0].pane_id, "%7");
    });
}
#[test]
fn processed_hook_events_are_deduplicated_across_channels() {
    let event = HookEvent {
        event: "stop".into(),
        turn_id: Some("turn-1".into()),
        session_id: Some("$1".into()),
        transcript_path: None,
        cwd: None,
        prompt: Some("hello".into()),
        last_assistant_message: Some("done".into()),
        timestamp: Some("2026-03-26T03:38:06Z".into()),
        tmux: HookTmuxInfo {
            pane_id: Some("%14".into()),
            session_name: Some("0".into()),
            window_index: Some("2".into()),
            pane_index: Some("1".into()),
            pane_current_path: Some("/tmp".into()),
        },
    };

    let mut state = TelegramState::default();
    assert!(remember_processed_hook_event(&mut state, &event));
    assert!(!remember_processed_hook_event(&mut state, &event));
    assert_eq!(state.processed_hook_signatures.len(), 1);
}
#[test]
fn next_request_and_draft_ids_are_unique() {
    assert_ne!(next_request_id(), next_request_id());
    assert_ne!(next_draft_id(), next_draft_id());
}
#[test]
fn pending_lookup_is_per_pane_not_global() {
    let state = TelegramState {
        pending_requests: vec![sample_pending("tg-1", "%1", "awaiting_submit")],
        ..TelegramState::default()
    };

    assert_eq!(pending_request_index_by_pane(&state, "%1"), Some(0));
    assert_eq!(pending_request_index_by_pane(&state, "%2"), None);
}
#[test]
fn selected_target_reset_removes_only_matching_pending() {
    let mut state = TelegramState {
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

    let removed = remove_selected_target_pending_request(&mut state).expect("removed pending");
    assert_eq!(removed.request_id, "tg-2");
    assert_eq!(state.pending_requests.len(), 1);
    assert_eq!(state.pending_requests[0].request_id, "tg-1");
}
