use crate::model::{AgentPanel, AgentType};

pub(in crate::session_cache) fn supports_cached_session(panel: &AgentPanel) -> bool {
    matches!(
        panel.agent_type,
        AgentType::Claude | AgentType::Codex | AgentType::Gemini | AgentType::OpenCode
    )
}

#[cfg(test)]
mod tests {
    use super::supports_cached_session;
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

    fn panel(agent_type: AgentType) -> AgentPanel {
        AgentPanel {
            session: "s".into(),
            window: "w".into(),
            window_index: "1".into(),
            pane: "0".into(),
            pane_id: "%1".into(),
            agent_type,
            working_dir: "/tmp".into(),
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
    fn gemini_is_supported_by_session_cache() {
        assert!(supports_cached_session(&panel(AgentType::Gemini)));
    }

    #[test]
    fn opencode_is_supported_by_session_cache() {
        assert!(supports_cached_session(&panel(AgentType::OpenCode)));
    }
}
