use crate::app::state::FocusTarget;
use crate::app::{
    App, PreviewDetailCache, ThreadPreviewCacheEntry, THREAD_PREVIEW_CACHE_MAX_ENTRIES,
};
use crate::model::{
    AgentPanel, AgentState, AgentType, PreviewSource, PreviewTurn, PreviewView, SessionCacheState,
};
use crate::preview_source::PreviewUpdate;
use crate::sidebar::ThreadActivityOverride;
use ratatui::text::Line;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn send_preview_update(app: &mut App, update: PreviewUpdate) {
    let (tx, rx) = mpsc::channel(1);
    tx.blocking_send(update).unwrap();
    app.preview.rx = Some(rx);
    app.check_preview_result();
}

pub(crate) mod latest {
    use super::*;
    pub(crate) fn open_latest_preview_turn_uses_selected_panel_cached_turns() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "main".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            is_active: true,
            state: AgentState::Busy,
            transcript_path: None,
            cached_preview_turns: vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("- item".into()),
            }]
            .into(),
            session_cache_state: Some(SessionCacheState::Cached),
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });

        app.preview.source = PreviewSource::Session;
        app.preview.pane_id = Some("%other".into());
        app.preview.turns = vec![PreviewTurn {
            question: "stale".into(),
            answer: Some("stale".into()),
        }]
        .into();

        assert!(app.open_latest_preview_turn());
        assert_eq!(app.preview.pane_id.as_deref(), Some("live:%1"));
        assert_eq!(app.preview.selected_turn, Some(0));
        assert_eq!(app.preview.expanded_turn, Some(0));
        assert_eq!(app.preview.turns[0].question, "latest");
    }

    pub(crate) fn open_latest_preview_turn_prefers_newer_panel_cached_turns_over_current_preview() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "main".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/demo".into(),
            is_active: true,
            state: AgentState::Busy,
            transcript_path: None,
            cached_preview_turns: vec![PreviewTurn {
                question: "new prompt".into(),
                answer: None,
            }]
            .into(),
            session_cache_state: Some(SessionCacheState::Confirmed),
            agent_session_id: Some("session-1".into()),
            last_user_prompt: Some("new prompt".into()),
            last_assistant_message: None,
            has_unread_stop: false,
        });
        app.table_state.select(Some(0));

        app.preview.source = PreviewSource::Session;
        app.preview.pane_id = Some("live:%1".into());
        app.preview.session_id = Some("session-1".into());
        app.preview.turns = vec![PreviewTurn {
            question: "old prompt".into(),
            answer: Some("old answer".into()),
        }]
        .into();

        assert!(app.open_latest_preview_turn());
        assert_eq!(
            app.preview.turns.first().map(|turn| turn.question.as_str()),
            Some("new prompt")
        );
        assert_eq!(
            app.preview
                .turns
                .first()
                .and_then(|turn| turn.answer.as_deref()),
            None
        );
    }
}

pub(crate) mod cache_dirty {
    use super::*;
    pub(crate) mod detail_cache {
        use super::*;
        pub(crate) fn detail_view_keeps_background_preview_refresh_alive() {
            let mut app = App::new();
            app.preview.source = PreviewSource::Session;
            app.preview.turns = vec![
                PreviewTurn {
                    question: "latest".into(),
                    answer: Some("latest answer".into()),
                },
                PreviewTurn {
                    question: "older".into(),
                    answer: Some("older answer".into()),
                },
            ]
            .into();
            app.preview.selected_turn = Some(1);
            app.preview.expanded_turn = Some(1);
            app.preview.view = PreviewView::SessionDetail;

            assert!(!app.should_pause_preview_refresh());

            app.preview.selected_turn = Some(0);
            app.preview.expanded_turn = Some(0);
            assert!(!app.should_pause_preview_refresh());
        }

        pub(crate) fn identical_preview_update_preserves_detail_cache() {
            let mut app = App::new();
            let turns = vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("latest answer".into()),
            }];
            app.preview.source = PreviewSource::Session;
            app.preview.pane_id = Some("live:%1".into());
            app.preview.turns = turns.clone().into();
            app.preview.selected_turn = Some(0);
            app.preview.expanded_turn = Some(0);
            app.preview.detail_cache = Some(PreviewDetailCache {
                target_key: "live:%1".into(),
                turns: app.preview.turns.clone(),
                turn_index: 0,
                width: 80,
                theme_name: "matrix".into(),
                question: "latest".into(),
                answer: Some("latest answer".into()),
                lines: vec![Line::from("cached")],
            });

            let (tx, rx) = mpsc::channel(1);
            tx.blocking_send(PreviewUpdate {
                target_key: "live:%1".into(),
                live_pane_id: Some("%1".into()),
                content: "latest\nlatest answer".into(),
                source: PreviewSource::Session,
                session_origin: None,
                session_id: Some("session-1".into()),
                turns: turns.into(),
                transcript_path: None,
                session_cache_state: Some(SessionCacheState::Cached),
                updated_at: None,
            })
            .unwrap();
            app.preview.rx = Some(rx);

            app.check_preview_result();

            assert!(app.preview.detail_cache.is_some());
            assert_eq!(
                app.preview
                    .detail_cache
                    .as_ref()
                    .and_then(|cache| cache.lines.first())
                    .map(|line| line.to_string()),
                Some("cached".to_string())
            );
        }

        pub(crate) fn matching_detail_cache_rebases_to_current_turn_allocation() {
            let mut app = App::new();
            let old_turns: crate::model::SharedPreviewTurns = vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("latest answer".into()),
            }]
            .into();
            let new_turns: crate::model::SharedPreviewTurns = vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("latest answer".into()),
            }]
            .into();
            app.preview.pane_id = Some("live:%1".into());
            app.preview.turns = old_turns.clone();
            app.preview.detail_cache = Some(PreviewDetailCache {
                target_key: "live:%1".into(),
                turns: old_turns,
                turn_index: 0,
                width: 80,
                theme_name: "matrix".into(),
                question: "latest".into(),
                answer: Some("latest answer".into()),
                lines: vec![Line::from("cached")],
            });
            app.preview.turns = new_turns;

            assert!(app
                .cached_preview_detail_for(
                    "live:%1",
                    0,
                    80,
                    "matrix",
                    "latest",
                    &Some("latest answer".into()),
                )
                .is_some());
            assert!(app
                .current_preview_detail_cache_for_current_turns("live:%1", 0, 80, "matrix")
                .is_some());
        }
    }

    pub(crate) mod dirty_update {
        use super::*;
        pub(crate) fn identical_preview_update_keeps_dirty_cleared() {
            let mut app = App::new();
            let turns = vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("latest answer".into()),
            }];
            app.preview.content = "latest\nlatest answer".into();
            app.preview.source = PreviewSource::Session;
            app.preview.pane_id = Some("live:%1".into());
            app.preview.session_origin = Some(crate::model::PreviewSessionOrigin::Pane);
            app.preview.session_id = Some("session-1".into());
            app.preview.turns = turns.clone().into();
            app.preview.selected_turn = Some(0);
            app.preview.expanded_turn = Some(0);
            app.preview.view = PreviewView::SessionDetail;
            app.preview.follow_bottom = false;
            app.preview.follow_selection = true;
            app.dirty = false;

            send_preview_update(
                &mut app,
                PreviewUpdate {
                    target_key: "live:%1".into(),
                    live_pane_id: Some("%1".into()),
                    content: "latest\nlatest answer".into(),
                    source: PreviewSource::Session,
                    session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
                    session_id: Some("session-1".into()),
                    turns: turns.into(),
                    transcript_path: None,
                    session_cache_state: Some(SessionCacheState::Confirmed),
                    updated_at: Some(42),
                },
            );

            assert!(!app.dirty);
        }

        pub(crate) fn preview_update_marks_dirty_when_content_changes_but_turns_do_not() {
            let mut app = App::new();
            let turns = vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("latest answer".into()),
            }];
            app.preview.content = "old".into();
            app.preview.source = PreviewSource::Session;
            app.preview.pane_id = Some("live:%1".into());
            app.preview.session_origin = Some(crate::model::PreviewSessionOrigin::Pane);
            app.preview.session_id = Some("session-1".into());
            app.preview.turns = turns.clone().into();
            app.preview.selected_turn = Some(0);
            app.preview.expanded_turn = Some(0);
            app.preview.view = PreviewView::SessionDetail;
            app.preview.follow_bottom = false;
            app.preview.follow_selection = true;
            app.dirty = false;

            send_preview_update(
                &mut app,
                PreviewUpdate {
                    target_key: "live:%1".into(),
                    live_pane_id: Some("%1".into()),
                    content: "new".into(),
                    source: PreviewSource::Session,
                    session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
                    session_id: Some("session-1".into()),
                    turns: turns.into(),
                    transcript_path: None,
                    session_cache_state: Some(SessionCacheState::Confirmed),
                    updated_at: Some(42),
                },
            );

            assert!(app.dirty);
        }

        pub(crate) fn preview_update_marks_dirty_when_turns_change_but_content_does_not() {
            let mut app = App::new();
            app.preview.content = "shared-content".into();
            app.preview.source = PreviewSource::Session;
            app.preview.pane_id = Some("live:%1".into());
            app.preview.session_origin = Some(crate::model::PreviewSessionOrigin::Pane);
            app.preview.session_id = Some("session-1".into());
            app.preview.turns = vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("old answer".into()),
            }]
            .into();
            app.preview.selected_turn = Some(0);
            app.preview.expanded_turn = Some(0);
            app.preview.view = PreviewView::SessionDetail;
            app.preview.follow_bottom = false;
            app.preview.follow_selection = true;
            app.dirty = false;

            send_preview_update(
                &mut app,
                PreviewUpdate {
                    target_key: "live:%1".into(),
                    live_pane_id: Some("%1".into()),
                    content: "shared-content".into(),
                    source: PreviewSource::Session,
                    session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
                    session_id: Some("session-1".into()),
                    turns: vec![PreviewTurn {
                        question: "latest".into(),
                        answer: Some("new answer".into()),
                    }]
                    .into(),
                    transcript_path: None,
                    session_cache_state: Some(SessionCacheState::Confirmed),
                    updated_at: Some(42),
                },
            );

            assert!(app.dirty);
            assert_eq!(app.preview.selected_turn, Some(0));
            assert_eq!(app.preview.expanded_turn, Some(0));
            assert_eq!(app.preview.view, PreviewView::SessionDetail);
        }
    }
}

pub(crate) mod selection_scroll {
    use super::*;
    pub(crate) mod context {
        use super::*;
        pub(crate) fn preview_update_context_change_resets_selection_and_scroll() {
            let mut app = App::new();
            app.preview.source = PreviewSource::Session;
            app.preview.pane_id = Some("live:%1".into());
            app.preview.session_origin = Some(crate::model::PreviewSessionOrigin::Pane);
            app.preview.session_id = Some("session-1".into());
            app.preview.turns = vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("latest answer".into()),
            }]
            .into();
            app.preview.selected_turn = Some(0);
            app.preview.expanded_turn = Some(0);
            app.preview.view = PreviewView::SessionDetail;
            app.preview.list_scroll = 4;
            app.preview.detail_scroll = 9;
            app.preview.follow_selection = false;
            app.preview.follow_bottom = false;

            send_preview_update(
                &mut app,
                PreviewUpdate {
                    target_key: "live:%2".into(),
                    live_pane_id: Some("%2".into()),
                    content: "another".into(),
                    source: PreviewSource::Session,
                    session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
                    session_id: Some("session-2".into()),
                    turns: vec![PreviewTurn {
                        question: "another".into(),
                        answer: Some("answer".into()),
                    }]
                    .into(),
                    transcript_path: None,
                    session_cache_state: Some(SessionCacheState::Confirmed),
                    updated_at: Some(43),
                },
            );

            assert_eq!(app.preview.selected_turn, None);
            assert_eq!(app.preview.expanded_turn, None);
            assert_eq!(app.preview.view, PreviewView::SessionList);
            assert_eq!(app.preview.list_scroll, 0);
            assert_eq!(app.preview.detail_scroll, 0);
            assert!(app.preview.follow_selection);
        }
    }

    pub(crate) mod detail {
        use super::*;
        pub(crate) fn preview_update_same_context_preserves_detail_selection_and_scroll() {
            let mut app = App::new();
            app.preview.content = "before".into();
            app.preview.source = PreviewSource::Session;
            app.preview.pane_id = Some("live:%1".into());
            app.preview.session_origin = Some(crate::model::PreviewSessionOrigin::Pane);
            app.preview.session_id = Some("session-1".into());
            app.preview.turns = vec![
                PreviewTurn {
                    question: "latest".into(),
                    answer: Some("latest answer".into()),
                },
                PreviewTurn {
                    question: "older".into(),
                    answer: Some("older answer".into()),
                },
            ]
            .into();
            app.preview.selected_turn = Some(1);
            app.preview.expanded_turn = Some(1);
            app.preview.view = PreviewView::SessionDetail;
            app.preview.list_scroll = 4;
            app.preview.detail_scroll = 9;
            app.preview.follow_bottom = false;
            app.preview.follow_selection = false;
            app.dirty = false;

            send_preview_update(
                &mut app,
                PreviewUpdate {
                    target_key: "live:%1".into(),
                    live_pane_id: Some("%1".into()),
                    content: "after".into(),
                    source: PreviewSource::Session,
                    session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
                    session_id: Some("session-1".into()),
                    turns: vec![
                        PreviewTurn {
                            question: "latest".into(),
                            answer: Some("latest answer".into()),
                        },
                        PreviewTurn {
                            question: "older".into(),
                            answer: Some("refreshed older answer".into()),
                        },
                    ]
                    .into(),
                    transcript_path: None,
                    session_cache_state: Some(SessionCacheState::Confirmed),
                    updated_at: Some(43),
                },
            );

            assert!(app.dirty);
            assert_eq!(app.preview.selected_turn, Some(1));
            assert_eq!(app.preview.expanded_turn, Some(1));
            assert_eq!(app.preview.view, PreviewView::SessionDetail);
            assert_eq!(app.preview.list_scroll, 4);
            assert_eq!(app.preview.detail_scroll, 9);
            assert!(!app.preview.follow_selection);
        }
        pub(crate) fn preview_update_same_context_clamps_selection_when_turns_shrink() {
            let mut app = App::new();
            app.preview.source = PreviewSource::Session;
            app.preview.pane_id = Some("live:%1".into());
            app.preview.session_origin = Some(crate::model::PreviewSessionOrigin::Pane);
            app.preview.session_id = Some("session-1".into());
            app.preview.turns = vec![
                PreviewTurn {
                    question: "latest".into(),
                    answer: Some("latest answer".into()),
                },
                PreviewTurn {
                    question: "older".into(),
                    answer: Some("older answer".into()),
                },
            ]
            .into();
            app.preview.selected_turn = Some(1);
            app.preview.expanded_turn = Some(1);
            app.preview.view = PreviewView::SessionDetail;
            app.preview.follow_bottom = false;
            app.preview.follow_selection = true;

            send_preview_update(
                &mut app,
                PreviewUpdate {
                    target_key: "live:%1".into(),
                    live_pane_id: Some("%1".into()),
                    content: "latest\nlatest answer".into(),
                    source: PreviewSource::Session,
                    session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
                    session_id: Some("session-1".into()),
                    turns: vec![PreviewTurn {
                        question: "latest".into(),
                        answer: Some("latest answer".into()),
                    }]
                    .into(),
                    transcript_path: None,
                    session_cache_state: Some(SessionCacheState::Confirmed),
                    updated_at: Some(42),
                },
            );

            assert_eq!(app.preview.selected_turn, None);
            assert_eq!(app.preview.expanded_turn, None);
            assert_eq!(app.preview.view, PreviewView::SessionList);
        }
    }

    pub(crate) mod panel_cache {
        use super::*;
        pub(crate) fn preview_update_marks_dirty_when_only_panel_cache_state_changes() {
            let mut app = App::new();
            app.panels.push(AgentPanel {
                session: "0".into(),
                window: "main".into(),
                window_index: "1".into(),
                pane: "1".into(),
                pane_id: "%1".into(),
                agent_type: AgentType::Codex,
                working_dir: "/tmp/demo".into(),
                is_active: true,
                state: AgentState::Idle,
                transcript_path: None,
                cached_preview_turns: Default::default(),
                session_cache_state: None,
                agent_session_id: None,
                last_user_prompt: None,
                last_assistant_message: None,
                has_unread_stop: false,
            });
            app.preview.content = "latest\nlatest answer".into();
            app.preview.source = PreviewSource::Session;
            app.preview.pane_id = Some("live:%1".into());
            app.preview.session_origin = Some(crate::model::PreviewSessionOrigin::Pane);
            app.preview.session_id = Some("session-1".into());
            app.preview.turns = vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("latest answer".into()),
            }]
            .into();
            app.preview.view = PreviewView::SessionList;
            app.preview.follow_bottom = false;
            app.preview.follow_selection = true;
            app.dirty = false;

            send_preview_update(
                &mut app,
                PreviewUpdate {
                    target_key: "live:%1".into(),
                    live_pane_id: Some("%1".into()),
                    content: "latest\nlatest answer".into(),
                    source: PreviewSource::Session,
                    session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
                    session_id: Some("session-1".into()),
                    turns: vec![PreviewTurn {
                        question: "latest".into(),
                        answer: Some("latest answer".into()),
                    }]
                    .into(),
                    transcript_path: None,
                    session_cache_state: Some(SessionCacheState::Cached),
                    updated_at: Some(42),
                },
            );

            assert!(app.dirty);
            assert_eq!(
                app.panels[0].session_cache_state,
                Some(SessionCacheState::Cached)
            );
        }
    }

    pub(crate) mod plain {
        use super::*;
        pub(crate) fn preview_update_plain_view_follow_bottom_depends_on_target_change() {
            struct Case {
                name: &'static str,
                previous_pane: Option<&'static str>,
                target: &'static str,
                initial_follow_bottom: bool,
                expected_follow_bottom: bool,
            }

            let cases = vec![
                Case {
                    name: "same target keeps false",
                    previous_pane: Some("%1"),
                    target: "%1",
                    initial_follow_bottom: false,
                    expected_follow_bottom: false,
                },
                Case {
                    name: "target switch forces true",
                    previous_pane: Some("%1"),
                    target: "%2",
                    initial_follow_bottom: false,
                    expected_follow_bottom: true,
                },
                Case {
                    name: "existing true stays true",
                    previous_pane: Some("%1"),
                    target: "%1",
                    initial_follow_bottom: true,
                    expected_follow_bottom: true,
                },
                Case {
                    name: "missing previous target defaults true",
                    previous_pane: None,
                    target: "%1",
                    initial_follow_bottom: false,
                    expected_follow_bottom: true,
                },
            ];

            for case in cases {
                let mut app = App::new();
                app.preview.pane_id = case.previous_pane.map(|pane| pane.to_string());
                app.preview.source = PreviewSource::Plain;
                app.preview.view = PreviewView::Plain;
                app.preview.content = "before".into();
                app.preview.follow_bottom = case.initial_follow_bottom;
                app.preview.follow_selection = false;
                app.dirty = false;

                send_preview_update(
                    &mut app,
                    PreviewUpdate {
                        target_key: case.target.into(),
                        live_pane_id: Some(case.target.into()),
                        content: "after".into(),
                        source: PreviewSource::Plain,
                        session_origin: None,
                        session_id: None,
                        turns: Default::default(),
                        transcript_path: None,
                        session_cache_state: None,
                        updated_at: None,
                    },
                );

                assert_eq!(
                    app.preview.follow_bottom, case.expected_follow_bottom,
                    "{}",
                    case.name
                );
                assert_eq!(app.preview.view, PreviewView::Plain, "{}", case.name);
                assert!(app.preview.turns.is_empty(), "{}", case.name);
                assert!(app.preview.follow_selection, "{}", case.name);
            }
        }
    }
}

pub(crate) mod tick_cache {
    use super::*;
    include!("preview_tests/tick_cache.rs");
}
