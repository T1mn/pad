use super::{AgentPanel, AgentState, AgentStateSource, AgentType};

fn panel_with_dir(working_dir: &str) -> AgentPanel {
    AgentPanel {
        session: String::new(),
        window: String::new(),
        window_index: String::new(),
        pane: String::new(),
        pane_id: String::new(),
        agent_type: AgentType::Codex,
        working_dir: working_dir.to_string(),
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
    }
}

#[test]
fn shortened_path_uses_last_two_segments_without_trailing_slash() {
    let panel = panel_with_dir("/very/long/workspace/project/repo");

    assert_eq!(panel.shortened_path(24), "~/.../project/repo");
}

#[test]
fn shortened_path_keeps_trailing_slash_semantics() {
    let panel = panel_with_dir("/very/long/workspace/project/repo/");

    assert_eq!(panel.shortened_path(24), "~/.../repo/");
}
