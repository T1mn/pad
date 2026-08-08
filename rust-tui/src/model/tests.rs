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
            AgentType::from_processes("bash zsh terminal"),
            AgentType::Unknown
        );
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
