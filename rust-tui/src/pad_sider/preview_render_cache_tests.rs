use super::super::app::App;
use super::super::preview::{FilePreview, PreviewKind};
use ratatui::text::Line;

#[test]
fn set_file_preview_keeps_render_cache_when_only_scroll_changes() {
    let mut app = App::new(std::env::temp_dir(), None);
    app.set_file_preview(FilePreview::new(
        "a".into(),
        "body".into(),
        PreviewKind::Text,
    ));
    app.store_rendered_file_preview(80, vec![Line::from("body")]);

    let mut next = FilePreview::new("a".into(), "body".into(), PreviewKind::Text);
    next.scroll = 10;
    let revision = app.file_preview_revision;
    app.set_file_preview(next);

    assert_eq!(app.file_preview_revision, revision);
    assert!(app.rendered_file_preview_matches(80));
}

#[test]
fn set_file_preview_invalidates_render_cache_when_content_changes() {
    let mut app = App::new(std::env::temp_dir(), None);
    app.set_file_preview(FilePreview::new(
        "a".into(),
        "body".into(),
        PreviewKind::Text,
    ));
    app.store_rendered_file_preview(80, vec![Line::from("body")]);
    let revision = app.file_preview_revision;

    app.set_file_preview(FilePreview::new(
        "a".into(),
        "changed".into(),
        PreviewKind::Text,
    ));

    assert_ne!(app.file_preview_revision, revision);
    assert!(app.rendered_file_preview.is_none());
}
