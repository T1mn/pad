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
