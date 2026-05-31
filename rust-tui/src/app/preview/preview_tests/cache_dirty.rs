#[test]
fn detail_view_keeps_background_preview_refresh_alive() {
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

#[test]
fn identical_preview_update_preserves_detail_cache() {
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

#[test]
fn matching_detail_cache_rebases_to_current_turn_allocation() {
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

#[test]
fn identical_preview_update_keeps_dirty_cleared() {
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

#[test]
fn preview_update_marks_dirty_when_content_changes_but_turns_do_not() {
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

#[test]
fn preview_update_marks_dirty_when_turns_change_but_content_does_not() {
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
