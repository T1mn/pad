#[cfg(test)]
mod lookup {
    use super::super::bindings::load_snapshots_for_agent_type;
    use super::super::model::{CachedSessionRecord, SessionCacheIndex};
    use crate::model::{PreviewTurn, SessionCacheState};

    #[test]
    fn loads_only_requested_agent_snapshots_with_source_state() {
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![
                record("codex", "codex", "resolver"),
                record("claude", "claude", "hook"),
            ],
        };

        let snapshots = load_snapshots_for_agent_type(&index, "codex");

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots["codex"].state, SessionCacheState::Confirmed);
        assert_eq!(snapshots["codex"].recent_turns[0].question, "question");
    }

    fn record(id: &str, agent_type: &str, source: &str) -> CachedSessionRecord {
        CachedSessionRecord {
            agent_session_id: id.to_string(),
            agent_type: agent_type.to_string(),
            transcript_path: Some(format!("/tmp/{id}.jsonl")),
            recent_turns: vec![PreviewTurn {
                question: "question".to_string(),
                answer: Some("answer".to_string()),
            }],
            last_user_prompt: Some("question".to_string()),
            last_assistant_message: Some("answer".to_string()),
            last_seen_at: 1,
            updated_at: 1,
            last_source: source.to_string(),
        }
    }
}

#[cfg(test)]
mod persist {
    use super::support::panel;
    use crate::hook::{HookEvent, HookTerminalInfo};

    fn session_start(session_id: &str) -> HookEvent {
        HookEvent {
            event: "session_start".to_string(),
            turn_id: None,
            session_id: Some(session_id.to_string()),
            transcript_path: None,
            cwd: None,
            prompt: None,
            last_assistant_message: None,
            timestamp: None,
            terminal: HookTerminalInfo {
                pane_id: Some("%1".to_string()),
                session_name: None,
                window_index: None,
                pane_index: None,
                pane_current_path: None,
            },
        }
    }

    fn panel_with_session(session_id: &str) -> crate::model::AgentPanel {
        let mut panel = panel("%1", "dev", "1", "0", "/repo");
        panel.agent_session_id = Some(session_id.to_string());
        panel.transcript_path = Some("/tmp/old.jsonl".to_string());
        panel.last_user_prompt = Some("old prompt".to_string());
        panel.last_assistant_message = Some("old answer".to_string());
        panel
    }

    #[test]
    fn session_start_for_new_id_does_not_inherit_panel_session_state() {
        crate::test_support::with_temp_home("pad-session-cache-persist", "new-session", |_| {
            let panel = panel_with_session("old");

            let snapshot = super::super::persist_hook_event(&panel, &session_start("new"))
                .unwrap()
                .unwrap();

            assert_eq!(snapshot.agent_session_id, "new");
            assert_eq!(snapshot.transcript_path, None);
            assert!(snapshot.recent_turns.is_empty());
            assert_eq!(snapshot.last_user_prompt, None);
            assert_eq!(snapshot.last_assistant_message, None);
        });
    }

    #[test]
    fn session_start_for_same_id_keeps_panel_fallbacks() {
        crate::test_support::with_temp_home("pad-session-cache-persist", "same-session", |_| {
            let panel = panel_with_session("same");

            let snapshot = super::super::persist_hook_event(&panel, &session_start("same"))
                .unwrap()
                .unwrap();

            assert_eq!(snapshot.transcript_path.as_deref(), Some("/tmp/old.jsonl"));
            assert_eq!(snapshot.recent_turns.len(), 1);
            assert_eq!(snapshot.recent_turns[0].question, "old prompt");
            assert_eq!(
                snapshot.recent_turns[0].answer.as_deref(),
                Some("old answer")
            );
            assert_eq!(snapshot.last_user_prompt.as_deref(), Some("old prompt"));
            assert_eq!(
                snapshot.last_assistant_message.as_deref(),
                Some("old answer")
            );
        });
    }
}
#[cfg(test)]
mod support {
    use crate::model::{AgentPanel, AgentState, AgentType};

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
            transcript_path: None,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        }
    }
}
#[cfg(test)]
mod turns {
    use super::super::turns::merge_recent_turns;
    use crate::model::PreviewTurn;

    #[test]
    fn merge_recent_turns_prefers_latest_prompt_and_answer() {
        let mut turns = Vec::new();
        merge_recent_turns(&mut turns, Some("hello"), None, None);
        merge_recent_turns(&mut turns, None, Some("world"), Some("hello"));
        assert_eq!(
            turns,
            vec![PreviewTurn {
                question: "hello".to_string(),
                answer: Some("world".to_string()),
            }]
        );
    }

    #[test]
    fn merge_recent_turns_does_not_reuse_previous_answer_for_new_prompt() {
        let mut turns = vec![PreviewTurn {
            question: "old prompt".to_string(),
            answer: Some("old answer".to_string()),
        }];

        merge_recent_turns(&mut turns, Some("new prompt"), None, Some("new prompt"));

        assert_eq!(
            turns,
            vec![
                PreviewTurn {
                    question: "new prompt".to_string(),
                    answer: None,
                },
                PreviewTurn {
                    question: "old prompt".to_string(),
                    answer: Some("old answer".to_string()),
                },
            ]
        );
    }
}
