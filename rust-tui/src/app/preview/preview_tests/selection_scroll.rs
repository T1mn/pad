#[test]
fn preview_update_same_context_preserves_detail_selection_and_scroll() {
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

#[test]
fn preview_update_plain_view_follow_bottom_depends_on_target_change() {
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
        app.preview.source = PreviewSource::Tmux;
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
                source: PreviewSource::Tmux,
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

#[test]
fn preview_update_marks_dirty_when_only_panel_cache_state_changes() {
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

#[test]
fn preview_update_same_context_clamps_selection_when_turns_shrink() {
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

#[test]
fn preview_update_context_change_resets_selection_and_scroll() {
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

