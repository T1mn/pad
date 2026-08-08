use super::{
    approval_callback_data, build_agent_keyboard, build_help_keyboard, build_slash_command_text,
    callbacks::approval_pending_index,
    chunk_text,
    commands::{
        build_pad_restart_shell_command, build_pad_status_body, format_recent_history_message,
        recent_history_turns, select_pad_restart_target, PadRestartTarget,
    },
    help_page_html,
    hooks::matching_pending_request_index,
    mark_update_processed, parse_approval_callback_data,
    pending::{completed_reply_text, pending_status_summary_line},
    pending_status_text, play_sound_event, remember_processed_hook_event,
    scan_codex_approval_updates, should_probe_hook_journal_inner,
    state::{
        load_state, next_draft_id, next_request_id, pending_request_index_by_pane,
        remove_selected_target_pending_request,
    },
    summarize_pane_capture, CodexApprovalRequest, HelpPage, PendingRequest, SelectedTarget,
    TelegramState,
};
use crate::hook::{HookEvent, HookTmuxInfo};
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType, PreviewTurn};
use crate::tmux_dispatch::SessionPaneInfo;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn sample_pending(request_id: &str, pane_id: &str, phase: &str) -> PendingRequest {
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
        failure_scan_offset: 0,
        last_failure_check_at: None,
        approval_scan_offset: 0,
        approval_call_id: None,
        approval_justification: None,
        completed_text: None,
        completed_source: None,
        delivery_attempts: 0,
        delivery_retry_at: 0,
    }
}

pub(super) fn sample_panel_with_turns(turns: Vec<PreviewTurn>) -> AgentPanel {
    AgentPanel {
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
        cached_preview_turns: turns.into(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: Some("session-42".into()),
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    }
}

fn temp_home(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("pad-telegram-tests-{name}-{stamp}"))
}

pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
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
mod approval;
mod core {
    use super::*;

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
    fn summarize_pane_capture_keeps_last_eighteen_non_edge_lines() {
        let capture = format!(
            "\n{}\n\n",
            (1..=20)
                .map(|idx| format!("line {idx}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let expected = (3..=20)
            .map(|idx| format!("line {idx}"))
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(summarize_pane_capture(&capture), expected);
    }

    #[test]
    fn agent_keyboard_uses_clickable_use_callbacks() {
        let panel = sample_panel_with_turns(Vec::new());
        let keyboard = build_agent_keyboard(&[panel], crate::i18n::Locale::En);
        assert_eq!(keyboard.len(), 1);
        assert_eq!(keyboard[0][0]["callback_data"], "use-pane:%42");
    }
    #[test]
    fn telegram_sound_helper_records_enabled_event() {
        with_temp_home("telegram-sound", |_home| {
            crate::sound::with_test_sound_capture(|| {
                let _ = crate::sound::take_test_playbacks();
                let mut config = crate::theme::Config::default();
                config.sound.approval.enabled = true;

                play_sound_event(&config, crate::sound::SoundEvent::Approval);

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
}
mod help {
    use super::*;

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
        let codex_html = help_page_html(crate::i18n::Locale::En, &state, HelpPage::Codex);
        assert!(codex_html.contains("Pad Telegram"));
        assert!(codex_html.contains("X rust-tui"));
        assert!(codex_html.contains("/status"));
        assert!(codex_html.contains("/compact"));

        let overview_html = help_page_html(crate::i18n::Locale::En, &state, HelpPage::Overview);
        assert!(overview_html.contains("/history"));
        assert!(overview_html.contains("/diag"));
        assert!(overview_html.contains("/restart"));
        assert!(overview_html.contains("/reset"));
    }

    #[test]
    fn help_page_html_escapes_target_label() {
        let state = TelegramState {
            selected_target: Some(SelectedTarget {
                pane_id: "%7".into(),
                label: "A&B <codex> 東".into(),
            }),
            ..TelegramState::default()
        };

        let html = help_page_html(crate::i18n::Locale::En, &state, HelpPage::Overview);
        assert!(html.contains("A&amp;B &lt;codex&gt; 東"));
        assert!(!html.contains("A&B <codex> 東"));
    }

    #[test]
    fn help_keyboard_marks_active_page() {
        let keyboard = build_help_keyboard(crate::i18n::Locale::En, HelpPage::Workflow);
        assert_eq!(keyboard.len(), 2);
        assert_eq!(keyboard[0][2]["callback_data"], "help:workflow");
        assert_eq!(keyboard[1][0]["callback_data"], "help:list");
    }
}
mod history_restart;
mod journal {
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
}
mod pending;
mod state;
