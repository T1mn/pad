#[test]
fn preview_update_identical_plain_view_preserves_plain_cache() {
    let mut app = App::new();
    app.preview.source = PreviewSource::Tmux;
    app.preview.view = PreviewView::Plain;
    app.preview.pane_id = Some("%1".into());
    app.preview.content = "plain".into();
    app.preview.plain_cache = Some(crate::app::PreviewPlainCache {
        target_key: "%1".into(),
        width: 80,
        theme_name: app.theme.name.to_string(),
        content_revision: app.preview.content_revision,
        lines: vec![Line::from("plain")],
        wrapped_rows: 1,
    });

    send_preview_update(
        &mut app,
        PreviewUpdate {
            target_key: "%1".into(),
            live_pane_id: Some("%1".into()),
            content: "plain".into(),
            source: PreviewSource::Tmux,
            session_origin: None,
            session_id: None,
            turns: Default::default(),
            transcript_path: None,
            session_cache_state: None,
            updated_at: None,
        },
    );

    assert!(app.preview.plain_cache.is_some());
}

#[test]
fn preview_update_changed_plain_content_bumps_revision_and_drops_cache() {
    let mut app = App::new();
    app.preview.source = PreviewSource::Tmux;
    app.preview.view = PreviewView::Plain;
    app.preview.pane_id = Some("%1".into());
    app.preview.content = "plain".into();
    app.preview.plain_cache = Some(crate::app::PreviewPlainCache {
        target_key: "%1".into(),
        width: 80,
        theme_name: app.theme.name.to_string(),
        content_revision: app.preview.content_revision,
        lines: vec![Line::from("plain")],
        wrapped_rows: 1,
    });
    let initial_revision = app.preview.content_revision;

    send_preview_update(
        &mut app,
        PreviewUpdate {
            target_key: "%1".into(),
            live_pane_id: Some("%1".into()),
            content: "changed".into(),
            source: PreviewSource::Tmux,
            session_origin: None,
            session_id: None,
            turns: Default::default(),
            transcript_path: None,
            session_cache_state: None,
            updated_at: None,
        },
    );

    assert_ne!(app.preview.content_revision, initial_revision);
    assert!(app.preview.plain_cache.is_none());
}
