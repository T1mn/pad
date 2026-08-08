mod agent {
    use super::super::agent::AgentType;

    #[test]
    fn from_processes_detects_agent_case_insensitively() {
        assert_eq!(
            AgentType::from_processes("/usr/bin/CODEX"),
            AgentType::Codex
        );
        assert_eq!(
            AgentType::from_processes("node OpenCode"),
            AgentType::OpenCode
        );
        assert_eq!(
            AgentType::from_processes("/Users/tim/.grok/downloads/grok-0.2.102-macos-aarch64"),
            AgentType::Grok
        );
    }

    #[test]
    fn from_processes_returns_unknown_without_agent_name() {
        assert_eq!(
            AgentType::from_processes("bash zsh tmux"),
            AgentType::Unknown
        );
    }
}

mod panel {
    use super::super::{AgentPanel, AgentState, AgentStateSource, AgentType};

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
}

mod preview {
    use super::super::preview::{PreviewTurn, SharedPreviewTurns};

    #[test]
    fn shared_preview_turns_clone_reuses_allocation() {
        let turns = SharedPreviewTurns::from(vec![PreviewTurn {
            question: "hello".into(),
            answer: Some("world".into()),
        }]);
        let cloned = turns.clone();

        assert!(turns.shares_allocation_with(&cloned));
        assert_eq!(cloned[0].question, "hello");
        assert_eq!(cloned[0].answer.as_deref(), Some("world"));
    }

    #[test]
    fn shared_preview_turns_equality_uses_same_allocation() {
        let turns = SharedPreviewTurns::from(vec![PreviewTurn {
            question: "hello".into(),
            answer: Some("world".into()),
        }]);
        let cloned = turns.clone();

        assert_eq!(turns, cloned);
    }
}
