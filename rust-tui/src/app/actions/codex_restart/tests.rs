use super::*;
use crate::model::{AgentPanel, AgentState, AgentStateSource};

fn assert_command_parts(command: &str, suffix: &str) {
    assert!(
        command.starts_with("exec '"),
        "missing PAD Codex runtime prefix: {command}"
    );
    assert!(
        !command.contains("CODEX_HOME="),
        "restart command must not override CODEX_HOME: {command}"
    );
    assert!(
        command.contains("/.pad/scripts/pad-codex'"),
        "missing pad-codex wrapper: {command}"
    );
    assert!(
        command.ends_with(suffix),
        "unexpected command suffix: {command}"
    );
}

fn test_panel(agent_type: AgentType, state: AgentState) -> AgentPanel {
    AgentPanel {
        session: "s".into(),
        window: "w".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type,
        working_dir: "/tmp".into(),
        is_active: false,
        state,
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
    }
}

#[test]
fn restart_preflight_does_not_block_non_idle_codex() {
    for state in [AgentState::Idle, AgentState::Busy, AgentState::Waiting] {
        let panel = test_panel(AgentType::Codex, state);
        assert!(codex_restart_preflight_message(&panel, Locale::ZhCN).is_none());
    }
}

#[test]
fn restart_preflight_still_blocks_non_codex() {
    let panel = test_panel(AgentType::Claude, AgentState::Idle);
    assert_eq!(
        codex_restart_preflight_message(&panel, Locale::ZhCN),
        Some("只支持 Codex 面板")
    );
}

#[test]
fn restart_command_resumes_specific_session() {
    assert_command_parts(
        &build_codex_restart_command("codex", "/tmp/project", Some("sid-1")),
        "/.pad/scripts/pad-codex' -C '/tmp/project' resume 'sid-1'",
    );
}

#[test]
fn restart_command_falls_back_to_last_session() {
    assert_command_parts(
        &build_codex_restart_command("codex", "/tmp/project", None),
        "/.pad/scripts/pad-codex' -C '/tmp/project' resume --last",
    );
}

#[test]
fn restart_command_quotes_shell_values() {
    assert_command_parts(
        &build_codex_restart_command("codex --profile work", "/tmp/a'b", Some("s'id")),
        r"/.pad/scripts/pad-codex' -C '/tmp/a'\''b' resume 's'\''id'",
    );
}
