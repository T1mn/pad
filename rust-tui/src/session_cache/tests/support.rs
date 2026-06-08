use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

pub(super) fn panel(
    pane_id: &str,
    session: &str,
    window_index: &str,
    pane: &str,
    path: &str,
) -> AgentPanel {
    AgentPanel {
        session: session.to_string(),
        window: "win".to_string(),
        window_index: window_index.to_string(),
        pane: pane.to_string(),
        pane_id: pane_id.to_string(),
        agent_type: AgentType::Codex,
        working_dir: path.to_string(),
        is_active: false,
        state: AgentState::Idle,
        state_source: AgentStateSource::Scanner,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: Some(format!("pid-{}", pane_id)),
        start_time: None,
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    }
}
