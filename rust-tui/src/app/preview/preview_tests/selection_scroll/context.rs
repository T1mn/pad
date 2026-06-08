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
