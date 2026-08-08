#[cfg(test)]
mod bindings {
    use super::super::bindings::find_snapshot_for_panel;
    use super::super::model::{
        snapshot_from_record, CachedPaneBinding, CachedSessionRecord, SessionCacheIndex,
    };
    use super::super::util::now_ts;
    use super::support::panel;
    use crate::model::{PreviewTurn, SessionCacheState};

    fn record(id: &str, transcript_path: &str, question: &str) -> CachedSessionRecord {
        CachedSessionRecord {
            agent_session_id: id.to_string(),
            agent_type: "codex".to_string(),
            transcript_path: Some(transcript_path.to_string()),
            recent_turns: vec![PreviewTurn {
                question: question.to_string(),
                answer: None,
            }],
            last_user_prompt: None,
            last_assistant_message: None,
            last_seen_at: 1,
            updated_at: 1,
            last_source: "hook".to_string(),
        }
    }

    fn binding(session_id: &str, pane_id: &str) -> CachedPaneBinding {
        CachedPaneBinding {
            agent_session_id: session_id.to_string(),
            pane_id: pane_id.to_string(),
            pane_pid: Some(format!("pid-{pane_id}")),
            session_name: "dev".to_string(),
            window_index: "1".to_string(),
            pane_index: "0".to_string(),
            path: "/repo".to_string(),
            agent_type: "codex".to_string(),
            updated_at: 1,
        }
    }

    #[test]
    fn fallback_match_is_ambiguous_when_multiple_sessions_share_same_slot() {
        let now = now_ts();
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![
                record("s1", "/tmp/a.jsonl", "q1"),
                record("s2", "/tmp/b.jsonl", "q2"),
            ],
            pane_bindings: vec![
                CachedPaneBinding {
                    updated_at: now,
                    ..binding("s1", "%1")
                },
                CachedPaneBinding {
                    updated_at: now,
                    ..binding("s2", "%2")
                },
            ],
        };

        assert!(find_snapshot_for_panel(&index, &panel("%9", "dev", "1", "0", "/repo")).is_none());
    }

    #[test]
    fn exact_pane_match_wins_even_if_slot_history_is_ambiguous() {
        let record = record("s1", "/tmp/a.jsonl", "q1");
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![record.clone()],
            pane_bindings: vec![binding("s1", "%1")],
        };

        let snapshot =
            find_snapshot_for_panel(&index, &panel("%1", "other", "9", "9", "/else")).unwrap();
        assert_eq!(
            snapshot,
            snapshot_from_record(&record, SessionCacheState::Cached)
        );
    }

    #[test]
    fn fallback_match_allows_duplicate_bindings_for_same_session_id() {
        let now = now_ts();
        let record = record("s1", "/tmp/a.jsonl", "q1");
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![record.clone()],
            pane_bindings: vec![
                CachedPaneBinding {
                    updated_at: now,
                    ..binding("s1", "%1")
                },
                CachedPaneBinding {
                    updated_at: now,
                    ..binding("s1", "%2")
                },
            ],
        };

        let snapshot = find_snapshot_for_panel(&index, &panel("%9", "dev", "1", "0", "/repo"));
        assert_eq!(
            snapshot,
            Some(snapshot_from_record(&record, SessionCacheState::Cached))
        );
    }

    #[test]
    fn exact_match_requires_recent_binding_when_pid_is_missing() {
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![record("s1", "/tmp/a.jsonl", "q1")],
            pane_bindings: vec![CachedPaneBinding {
                pane_pid: None,
                ..binding("s1", "%1")
            }],
        };

        assert!(find_snapshot_for_panel(&index, &panel("%1", "dev", "1", "0", "/repo")).is_none());
    }

    #[test]
    fn exact_match_keeps_working_for_stale_binding_when_pane_pid_matches() {
        let record = record("s1", "/tmp/a.jsonl", "q1");
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![record.clone()],
            pane_bindings: vec![CachedPaneBinding {
                session_name: "old".to_string(),
                window_index: "9".to_string(),
                pane_index: "9".to_string(),
                path: "/else".to_string(),
                ..binding("s1", "%1")
            }],
        };

        let snapshot =
            find_snapshot_for_panel(&index, &panel("%1", "dev", "1", "0", "/repo")).unwrap();
        assert_eq!(
            snapshot,
            snapshot_from_record(&record, SessionCacheState::Cached)
        );
    }
}
#[cfg(test)]
mod persist {
    use super::support::panel;
    use crate::hook::{HookEvent, HookTmuxInfo};

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
            tmux: HookTmuxInfo {
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
mod preload {
    use super::super::model::SessionCacheSnapshot;
    use super::super::preload::{
        apply_snapshot_to_panel, latest_turn_missing_answer, panel_needs_preload,
    };
    use super::support::panel;
    use crate::model::{
        AgentState, AgentStateSource, AgentType, PreviewTurn, SessionCacheState, SharedPreviewTurns,
    };

    #[test]
    fn latest_unanswered_turn_restores_busy_state() {
        let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
        let snapshot = SessionCacheSnapshot {
            agent_session_id: "s1".to_string(),
            transcript_path: Some("/tmp/a.jsonl".to_string()),
            recent_turns: vec![PreviewTurn {
                question: "still running".to_string(),
                answer: None,
            }]
            .into(),
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
            }]
            .into(),
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
    fn preload_index_is_needed_only_for_supported_empty_panels() {
        let mut empty = panel("%1", "dev", "1", "0", "/repo");
        assert!(panel_needs_preload(&empty));

        empty.agent_session_id = Some("session-1".to_string());
        assert!(!panel_needs_preload(&empty));

        let mut unsupported = panel("%2", "dev", "1", "1", "/repo");
        unsupported.agent_type = AgentType::Aider;
        assert!(!panel_needs_preload(&unsupported));
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
            }]
            .into(),
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

    #[test]
    fn apply_snapshot_to_panel_reuses_clean_codex_turns() {
        let mut restored_panel = panel("%1", "dev", "1", "0", "/repo");
        let recent_turns = SharedPreviewTurns::from(vec![PreviewTurn {
            question: "plain prompt".to_string(),
            answer: Some("plain answer".to_string()),
        }]);
        let snapshot = SessionCacheSnapshot {
            agent_session_id: "s1".to_string(),
            transcript_path: Some("/tmp/a.jsonl".to_string()),
            recent_turns: recent_turns.clone(),
            last_user_prompt: Some("plain prompt".to_string()),
            last_assistant_message: Some("plain answer".to_string()),
            state: SessionCacheState::Cached,
        };

        apply_snapshot_to_panel(&mut restored_panel, &snapshot);

        assert!(restored_panel
            .cached_preview_turns
            .shares_allocation_with(&recent_turns));
    }
}
#[cfg(test)]
mod summary {
    use super::super::model::{CachedPaneBinding, CachedSessionRecord, SessionCacheIndex};
    use crate::model::PreviewTurn;

    fn record(id: &str, updated_at: i64) -> CachedSessionRecord {
        CachedSessionRecord {
            agent_session_id: id.to_string(),
            agent_type: "codex".to_string(),
            transcript_path: Some(format!("/tmp/{id}.jsonl")),
            recent_turns: vec![PreviewTurn {
                question: format!("question {id}"),
                answer: None,
            }],
            last_user_prompt: Some(format!("prompt {id}")),
            last_assistant_message: Some(format!("answer {id}")),
            last_seen_at: updated_at,
            updated_at,
            last_source: "hook".to_string(),
        }
    }

    fn binding(session_id: &str, pane_id: &str, path: &str, updated_at: i64) -> CachedPaneBinding {
        CachedPaneBinding {
            agent_session_id: session_id.to_string(),
            pane_id: pane_id.to_string(),
            pane_pid: Some(format!("pid-{pane_id}")),
            session_name: "dev".to_string(),
            window_index: "1".to_string(),
            pane_index: "0".to_string(),
            path: path.to_string(),
            agent_type: "codex".to_string(),
            updated_at,
        }
    }

    fn with_index(index: &SessionCacheIndex, f: impl FnOnce()) {
        crate::test_support::with_temp_home("pad-session-summary", "index", |_| {
            let path = crate::paths::sessions_index_path();
            std::fs::create_dir_all(path.parent().expect("index parent"))
                .expect("create sessions dir");
            std::fs::write(
                &path,
                serde_json::to_string_pretty(index).expect("serialize index"),
            )
            .expect("write index");
            f();
        });
    }

    #[test]
    fn find_cached_session_returns_matching_summary_with_latest_binding() {
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![record("old", 1), record("target", 2)],
            pane_bindings: vec![
                binding("target", "%1", "/older", 10),
                binding("target", "%2", "/newer", 20),
                binding("old", "%3", "/old", 30),
            ],
        };

        with_index(&index, || {
            let summary = super::super::find_cached_session(" target ").expect("summary");

            assert_eq!(summary.agent_session_id, "target");
            assert_eq!(summary.working_dir.as_deref(), Some("/newer"));
            assert_eq!(summary.pane_id.as_deref(), Some("%2"));
            assert_eq!(summary.last_user_prompt.as_deref(), Some("prompt target"));
        });
    }

    #[test]
    fn list_cached_sessions_and_find_use_same_summary_shape() {
        let index = SessionCacheIndex {
            version: 1,
            sessions: vec![record("target", 2)],
            pane_bindings: vec![binding("target", "%1", "/repo", 10)],
        };

        with_index(&index, || {
            let listed = super::super::list_cached_sessions()
                .into_iter()
                .find(|session| session.agent_session_id == "target")
                .expect("listed summary");
            let found = super::super::find_cached_session("target").expect("found summary");

            assert_eq!(found, listed);
        });
    }
}
#[cfg(test)]
mod support {
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
