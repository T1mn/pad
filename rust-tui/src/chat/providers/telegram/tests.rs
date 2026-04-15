use super::{
    approval_callback_data, build_agent_keyboard, build_help_keyboard, build_slash_command_text,
    callbacks::approval_pending_index,
    chunk_text,
    commands::build_pad_status_body,
    help_page_html,
    hooks::matching_pending_request_index,
    mark_update_processed, parse_approval_callback_data,
    pending::{completed_reply_text, pending_status_summary_line},
    pending_status_text, remember_processed_hook_event, scan_codex_approval_updates,
    should_probe_hook_journal_inner,
    state::{load_state, next_draft_id, next_request_id, pending_request_index_by_pane},
    summarize_pane_capture, CodexApprovalRequest, HelpPage, PendingRequest, SelectedTarget,
    TelegramState,
};
use crate::hook::{HookEvent, HookTmuxInfo};
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_pending(request_id: &str, pane_id: &str, phase: &str) -> PendingRequest {
    PendingRequest {
        request_id: request_id.into(),
        chat_id: "1".into(),
        pane_id: pane_id.into(),
        agent_kind: "codex".into(),
        target_label: format!("CODEX • {}", pane_id.trim_start_matches('%')),
        session_id: Some(format!("session-{}", pane_id.trim_start_matches('%'))),
        working_dir: format!("/tmp/{}", pane_id.trim_start_matches('%')),
        prompt_text: "hi".into(),
        prompt_hash: format!("{:x}", md5::compute("hi".as_bytes())),
        turn_id: None,
        sent_at: 100,
        sent_at_ms: 100_000,
        accepted_at: None,
        accepted_at_ms: None,
        last_status_at: None,
        draft_id: 123,
        phase: phase.into(),
        transcript_path: None,
        result_scan_offset: 0,
        approval_scan_offset: 0,
        approval_call_id: None,
        approval_justification: None,
        completed_text: None,
        completed_source: None,
        delivery_attempts: 0,
        delivery_retry_at: 0,
    }
}

fn temp_home(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("pad-telegram-tests-{name}-{stamp}"))
}

fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock telegram tests");
    let home = temp_home(name);
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let result = f(&home);

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);

    result
}

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
fn telegram_sound_helper_records_enabled_event() {
    with_temp_home("telegram-sound", |_home| {
        crate::sound::with_test_sound_capture(|| {
            let _ = crate::sound::take_test_playbacks();
            let mut config = crate::theme::Config::default();
            config.sound.approval.enabled = true;

            super::play_sound_event(&config, crate::sound::SoundEvent::Approval);

            assert_eq!(
                crate::sound::take_test_playbacks(),
                vec![crate::sound::TestPlayback {
                    event: Some(crate::sound::SoundEvent::Approval),
                    preset: "ping".into(),
                }]
            );
        });
    });
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
