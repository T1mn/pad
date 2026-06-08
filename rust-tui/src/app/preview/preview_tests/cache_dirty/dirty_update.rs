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
