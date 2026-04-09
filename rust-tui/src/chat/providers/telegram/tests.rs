use super::{
    approval_callback_data, build_agent_keyboard, build_help_keyboard, build_slash_command_text,
    chunk_text, help_page_html, mark_update_processed, parse_approval_callback_data,
    pending_status_text, remember_processed_hook_event, scan_codex_approval_updates,
    should_probe_hook_journal_inner, summarize_pane_capture, CodexApprovalRequest, HelpPage,
    PendingRequest, SelectedTarget, TelegramState,
};
use crate::hook::{HookEvent, HookTmuxInfo};
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
use std::fs;

#[test]
fn chunk_text_splits_long_messages() {
    let chunks = chunk_text("abcdef", 3);
    assert_eq!(chunks, vec!["abc", "def"]);
}

#[test]
fn slash_command_builder_preserves_optional_args() {
    assert_eq!(build_slash_command_text("/status", ""), "/status");
    assert_eq!(build_slash_command_text("/fast", "status"), "/fast status");
}

#[test]
fn summarize_pane_capture_trims_blank_edges_and_keeps_tail() {
    let capture = "\n\none\n\ntwo\nthree\n\n";
    assert_eq!(summarize_pane_capture(capture), "one\n\ntwo\nthree");
}

#[test]
fn agent_keyboard_uses_clickable_use_callbacks() {
    let panel = AgentPanel {
        session: "0".into(),
        window: "zsh".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%42".into(),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/rust-tui".into(),
        is_active: false,
        state: AgentState::Idle,
        state_source: AgentStateSource::Scanner,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    };
    let keyboard = build_agent_keyboard(&[panel], crate::i18n::Locale::En);
    assert_eq!(keyboard.len(), 1);
    assert_eq!(keyboard[0][0]["callback_data"], "use-pane:%42");
}

#[test]
fn pending_status_moves_from_accepted_to_working() {
    let pending = PendingRequest {
        request_id: "tg-1".into(),
        chat_id: "1".into(),
        pane_id: "%1".into(),
        agent_kind: "codex".into(),
        target_label: "CODEX • rust-tui".into(),
        prompt_text: "hi".into(),
        prompt_hash: "abc".into(),
        turn_id: None,
        sent_at: 100,
        sent_at_ms: 100_000,
        accepted_at: Some(100),
        accepted_at_ms: Some(100_000),
        last_status_at: None,
        draft_id: 123,
        phase: "awaiting_stop".into(),
        transcript_path: None,
        result_scan_offset: 0,
        approval_scan_offset: 0,
        approval_call_id: None,
        approval_justification: None,
        completed_text: None,
        completed_source: None,
        delivery_attempts: 0,
        delivery_retry_at: 0,
    };

    let accepted = pending_status_text(crate::i18n::Locale::En, &pending, 102);
    let working = pending_status_text(crate::i18n::Locale::En, &pending, 106);

    assert!(accepted.contains("Submitted"));
    assert!(working.contains("Working"));
    assert!(working.contains("6s"));
}

#[test]
fn pending_status_reports_approval_needed() {
    let pending = PendingRequest {
        request_id: "tg-1".into(),
        chat_id: "1".into(),
        pane_id: "%1".into(),
        agent_kind: "codex".into(),
        target_label: "CODEX • rust-tui".into(),
        prompt_text: "hi".into(),
        prompt_hash: "abc".into(),
        turn_id: None,
        sent_at: 100,
        sent_at_ms: 100_000,
        accepted_at: Some(100),
        accepted_at_ms: Some(100_000),
        last_status_at: None,
        draft_id: 123,
        phase: "awaiting_confirm".into(),
        transcript_path: None,
        result_scan_offset: 0,
        approval_scan_offset: 0,
        approval_call_id: Some("call_1".into()),
        approval_justification: Some("Do you want to allow running cargo check?".into()),
        completed_text: None,
        completed_source: None,
        delivery_attempts: 0,
        delivery_retry_at: 0,
    };

    let text = pending_status_text(crate::i18n::Locale::En, &pending, 110);
    assert!(text.contains("Needs approval"));
    assert!(text.contains("cargo check"));
}

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
}

#[test]
fn codex_approval_scan_tracks_open_and_resolved_requests() {
    let path =
        std::env::temp_dir().join(format!("pad-codex-approval-{}.jsonl", std::process::id()));
    let body = concat!(
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call_old\",\"arguments\":\"{\\\"sandbox_permissions\\\":\\\"require_escalated\\\",\\\"justification\\\":\\\"old\\\"}\"}}\n",
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"call_old\",\"output\":\"ok\"}}\n",
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call_new\",\"arguments\":\"{\\\"sandbox_permissions\\\":\\\"require_escalated\\\",\\\"justification\\\":\\\"new justification\\\"}\"}}\n"
    );
    fs::write(&path, body).unwrap();

    let result = scan_codex_approval_updates(&path, 0, None).unwrap();
    assert_eq!(
        result.active_request,
        Some(CodexApprovalRequest {
            call_id: "call_new".into(),
            justification: "new justification".into(),
        })
    );

    fs::write(
        &path,
        format!(
            "{}{}",
            body,
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"call_new\",\"output\":\"done\"}}\n"
        ),
    )
    .unwrap();
    let result =
        scan_codex_approval_updates(&path, result.next_offset, result.active_request).unwrap();
    assert!(result.active_request.is_none());

    let _ = fs::remove_file(path);
}

#[test]
fn help_page_callbacks_parse() {
    assert_eq!(
        HelpPage::from_callback("help:overview"),
        Some(HelpPage::Overview)
    );
    assert_eq!(HelpPage::from_callback("help:codex"), Some(HelpPage::Codex));
    assert_eq!(
        HelpPage::from_callback("help:workflow"),
        Some(HelpPage::Workflow)
    );
    assert_eq!(HelpPage::from_callback("help:list"), None);
}

#[test]
fn help_page_html_includes_target_and_commands() {
    let state = TelegramState {
        selected_target: Some(SelectedTarget {
            pane_id: "%7".into(),
            label: "X rust-tui".into(),
        }),
        ..TelegramState::default()
    };
    let html = help_page_html(crate::i18n::Locale::En, &state, HelpPage::Codex);
    assert!(html.contains("Pad Telegram"));
    assert!(html.contains("X rust-tui"));
    assert!(html.contains("/status"));
    assert!(html.contains("/compact"));
}

#[test]
fn help_keyboard_marks_active_page() {
    let keyboard = build_help_keyboard(crate::i18n::Locale::En, HelpPage::Workflow);
    assert_eq!(keyboard.len(), 2);
    assert_eq!(keyboard[0][2]["callback_data"], "help:workflow");
    assert_eq!(keyboard[1][0]["callback_data"], "help:list");
}

#[test]
fn approval_callback_data_round_trips_request_id_and_choice() {
    let data = approval_callback_data("tg-123", "a");
    assert_eq!(data, "approval:tg-123:a");
    assert_eq!(parse_approval_callback_data(&data), Some(("tg-123", "a")));
    assert_eq!(parse_approval_callback_data("approval:y"), None);
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
fn journal_recovery_runs_immediately_for_pending_on_startup() {
    let state = TelegramState {
        pending: Some(PendingRequest {
            request_id: "tg-1".into(),
            chat_id: "1".into(),
            pane_id: "%1".into(),
            agent_kind: "codex".into(),
            target_label: "CODEX • rust-tui".into(),
            prompt_text: "hi".into(),
            prompt_hash: "abc".into(),
            turn_id: None,
            sent_at: 100,
            sent_at_ms: 100_000,
            accepted_at: None,
            accepted_at_ms: None,
            last_status_at: None,
            draft_id: 123,
            phase: "awaiting_submit".into(),
            transcript_path: None,
            result_scan_offset: 0,
            approval_scan_offset: 0,
            approval_call_id: None,
            approval_justification: None,
            completed_text: None,
            completed_source: None,
            delivery_attempts: 0,
            delivery_retry_at: 0,
        }),
        ..TelegramState::default()
    };

    assert!(should_probe_hook_journal_inner(&state, true, 100));
}

#[test]
fn journal_recovery_waits_for_stall_when_direct_hook_is_alive() {
    let state = TelegramState {
        last_journal_recovery_at: 100,
        pending: Some(PendingRequest {
            request_id: "tg-1".into(),
            chat_id: "1".into(),
            pane_id: "%1".into(),
            agent_kind: "codex".into(),
            target_label: "CODEX • rust-tui".into(),
            prompt_text: "hi".into(),
            prompt_hash: "abc".into(),
            turn_id: None,
            sent_at: 101,
            sent_at_ms: 101_000,
            accepted_at: None,
            accepted_at_ms: None,
            last_status_at: None,
            draft_id: 123,
            phase: "awaiting_submit".into(),
            transcript_path: None,
            result_scan_offset: 0,
            approval_scan_offset: 0,
            approval_call_id: None,
            approval_justification: None,
            completed_text: None,
            completed_source: None,
            delivery_attempts: 0,
            delivery_retry_at: 0,
        }),
        ..TelegramState::default()
    };

    assert!(!should_probe_hook_journal_inner(&state, true, 103));
    assert!(should_probe_hook_journal_inner(&state, true, 106));
}

#[test]
fn scan_codex_answer_updates_ignores_old_messages_before_offset() {
    let path = std::env::temp_dir().join(format!("pad-codex-answer-{}.jsonl", std::process::id()));
    let first = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"old\"}]}}\n";
    let second = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"final_answer\",\"content\":[{\"type\":\"output_text\",\"text\":\"new\"}]}}\n";
    fs::write(&path, format!("{first}{second}")).unwrap();

    let result =
        crate::chat::approval::scan_codex_answer_updates(&path, first.len() as u64, None).unwrap();
    assert_eq!(result.as_deref(), Some("new"));

    let _ = fs::remove_file(path);
}

#[test]
fn scan_codex_answer_updates_ignores_commentary_until_task_complete() {
    let path = std::env::temp_dir().join(format!(
        "pad-codex-answer-task-complete-{}.jsonl",
        std::process::id()
    ));
    let commentary = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"intermediate\"}]}}\n";
    fs::write(&path, commentary).unwrap();

    let before_complete =
        crate::chat::approval::scan_codex_answer_updates(&path, 0, Some("turn-1")).unwrap();
    assert!(before_complete.is_none());

    let completed = concat!(
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"final_answer\",\"content\":[{\"type\":\"output_text\",\"text\":\"done\"}]}}\n",
        "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\",\"turn_id\":\"turn-1\",\"last_agent_message\":\"done\"}}\n"
    );
    fs::write(&path, format!("{commentary}{completed}")).unwrap();

    let after_complete =
        crate::chat::approval::scan_codex_answer_updates(&path, 0, Some("turn-1")).unwrap();
    assert_eq!(after_complete.as_deref(), Some("done"));

    let _ = fs::remove_file(path);
}
