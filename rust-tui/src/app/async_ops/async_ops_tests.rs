use super::*;

fn panel_with_state(state: AgentState, source: AgentStateSource) -> AgentPanel {
    AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type: crate::model::AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        is_active: matches!(state, AgentState::Busy),
        state,
        state_source: source,
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
fn only_busy_hook_state_is_preserved_across_scan() {
    assert!(should_preserve_hook_state(&panel_with_state(
        AgentState::Busy,
        AgentStateSource::Hook,
    )));
    assert!(!should_preserve_hook_state(&panel_with_state(
        AgentState::Waiting,
        AgentStateSource::Hook,
    )));
    assert!(!should_preserve_hook_state(&panel_with_state(
        AgentState::Idle,
        AgentStateSource::Hook,
    )));
    assert!(!should_preserve_hook_state(&panel_with_state(
        AgentState::Busy,
        AgentStateSource::Scanner,
    )));
}
