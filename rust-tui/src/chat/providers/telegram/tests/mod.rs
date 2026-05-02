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
mod core;
mod help;
mod history_restart;
mod journal;
mod pending;
mod state;
