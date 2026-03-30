#[cfg(test)]
mod cases {
    use super::super::bindings::find_snapshot_for_panel;
    use super::super::model::{
        snapshot_from_record, CachedPaneBinding, CachedSessionRecord, SessionCacheIndex,
        SessionCacheSnapshot,
    };
    use super::super::persist::merge_recent_turns;
    use super::super::preload::{apply_snapshot_to_panel, latest_turn_missing_answer};
    use crate::model::{
        AgentPanel, AgentState, AgentStateSource, AgentType, PreviewTurn, SessionCacheState,
    };

    fn panel(
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
            cached_preview_turns: Vec::new(),
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

    #[test]
    fn fallback_match_is_ambiguous_when_multiple_sessions_share_same_slot() {
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![
                CachedSessionRecord {
                    agent_session_id: "s1".to_string(),
                    agent_type: "codex".to_string(),
                    transcript_path: Some("/tmp/a.jsonl".to_string()),
                    recent_turns: vec![PreviewTurn {
                        question: "q1".to_string(),
                        answer: None,
                    }],
                    last_user_prompt: None,
                    last_assistant_message: None,
                    last_seen_at: 1,
                    updated_at: 1,
                    last_source: "hook".to_string(),
                },
                CachedSessionRecord {
                    agent_session_id: "s2".to_string(),
                    agent_type: "codex".to_string(),
                    transcript_path: Some("/tmp/b.jsonl".to_string()),
                    recent_turns: vec![PreviewTurn {
                        question: "q2".to_string(),
                        answer: None,
                    }],
                    last_user_prompt: None,
                    last_assistant_message: None,
                    last_seen_at: 1,
                    updated_at: 1,
                    last_source: "hook".to_string(),
                },
            ],
            pane_bindings: vec![
                CachedPaneBinding {
                    agent_session_id: "s1".to_string(),
                    pane_id: "%1".to_string(),
                    session_name: "dev".to_string(),
                    window_index: "1".to_string(),
                    pane_index: "0".to_string(),
                    path: "/repo".to_string(),
                    agent_type: "codex".to_string(),
                    updated_at: 1,
                },
                CachedPaneBinding {
                    agent_session_id: "s2".to_string(),
                    pane_id: "%2".to_string(),
                    session_name: "dev".to_string(),
                    window_index: "1".to_string(),
                    pane_index: "0".to_string(),
                    path: "/repo".to_string(),
                    agent_type: "codex".to_string(),
                    updated_at: 1,
                },
            ],
        };

        assert!(find_snapshot_for_panel(&index, &panel("%9", "dev", "1", "0", "/repo")).is_none());
    }

    #[test]
    fn exact_pane_match_wins_even_if_slot_history_is_ambiguous() {
        let record = CachedSessionRecord {
            agent_session_id: "s1".to_string(),
            agent_type: "codex".to_string(),
            transcript_path: Some("/tmp/a.jsonl".to_string()),
            recent_turns: vec![PreviewTurn {
                question: "q1".to_string(),
                answer: None,
            }],
            last_user_prompt: None,
            last_assistant_message: None,
            last_seen_at: 1,
            updated_at: 1,
            last_source: "hook".to_string(),
        };

        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![record.clone()],
            pane_bindings: vec![CachedPaneBinding {
                agent_session_id: "s1".to_string(),
                pane_id: "%1".to_string(),
                session_name: "dev".to_string(),
                window_index: "1".to_string(),
                pane_index: "0".to_string(),
                path: "/repo".to_string(),
                agent_type: "codex".to_string(),
                updated_at: 1,
            }],
        };

        let snapshot =
            find_snapshot_for_panel(&index, &panel("%1", "other", "9", "9", "/else")).unwrap();
        assert_eq!(
            snapshot,
            snapshot_from_record(&record, SessionCacheState::Cached)
        );
    }

    #[test]
    fn latest_unanswered_turn_restores_busy_state() {
        let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
        let snapshot = SessionCacheSnapshot {
            agent_session_id: "s1".to_string(),
            transcript_path: Some("/tmp/a.jsonl".to_string()),
            recent_turns: vec![PreviewTurn {
                question: "still running".to_string(),
                answer: None,
            }],
            last_user_prompt: Some("still running".to_string()),
            last_assistant_message: None,
            state: SessionCacheState::Cached,
        };

        apply_snapshot_to_panel(&mut restored_panel, &snapshot);

        assert_eq!(restored_panel.state, AgentState::Busy);
        assert_eq!(restored_panel.state_source, AgentStateSource::Hook);
        assert!(restored_panel.is_active);
    }

    #[test]
    fn answered_latest_turn_does_not_force_busy_state() {
        let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
        let snapshot = SessionCacheSnapshot {
            agent_session_id: "s1".to_string(),
            transcript_path: Some("/tmp/a.jsonl".to_string()),
            recent_turns: vec![PreviewTurn {
                question: "done".to_string(),
                answer: Some("finished".to_string()),
            }],
            last_user_prompt: Some("done".to_string()),
            last_assistant_message: Some("finished".to_string()),
            state: SessionCacheState::Cached,
        };

        apply_snapshot_to_panel(&mut restored_panel, &snapshot);

        assert_eq!(restored_panel.state, AgentState::Idle);
        assert_eq!(restored_panel.state_source, AgentStateSource::Scanner);
        assert!(!restored_panel.is_active);
    }

    #[test]
    fn latest_turn_missing_answer_only_when_newest_turn_is_unresolved() {
        assert!(latest_turn_missing_answer(&[PreviewTurn {
            question: "pending".to_string(),
            answer: None,
        }]));
        assert!(!latest_turn_missing_answer(&[PreviewTurn {
            question: "done".to_string(),
            answer: Some("answer".to_string()),
        }]));
        assert!(!latest_turn_missing_answer(&[
            PreviewTurn {
                question: "done".to_string(),
                answer: Some("answer".to_string()),
            },
            PreviewTurn {
                question: "old pending".to_string(),
                answer: None,
            },
        ]));
    }

    #[test]
    fn apply_snapshot_to_panel_normalizes_old_codex_image_placeholders() {
        let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
        let snapshot = SessionCacheSnapshot {
            agent_session_id: "s1".to_string(),
            transcript_path: Some("/tmp/a.jsonl".to_string()),
            recent_turns: vec![PreviewTurn {
                question: "<image name=[Image #1]>\n</image>\n[Image #1] 为什么有黑边？"
                    .to_string(),
                answer: Some("因为边框".to_string()),
            }],
            last_user_prompt: Some(
                "<image name=[Image #1]>\n</image>\n[Image #1] 为什么有黑边？".to_string(),
            ),
            last_assistant_message: Some("因为边框".to_string()),
            state: SessionCacheState::Cached,
        };

        apply_snapshot_to_panel(&mut restored_panel, &snapshot);

        assert_eq!(
            restored_panel.cached_preview_turns[0].question,
            "[Image x1] 为什么有黑边？"
        );
        assert_eq!(
            restored_panel.last_user_prompt.as_deref(),
            Some("[Image x1] 为什么有黑边？")
        );
    }
}
