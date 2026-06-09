use super::{extract_preview_selection_text, preview_visible_plain_text_rows};
use crate::app::App;
use crate::model::{PreviewSource, PreviewView};
use ratatui::layout::Rect;

#[test]
fn preview_plain_visible_rows_respects_scroll_window_after_wrapping() {
    let mut app = App::new();
    app.preview.source = PreviewSource::Tmux;
    app.preview.view = PreviewView::Plain;
    app.preview.pane_id = Some("%1".into());
    app.preview.content = "abcd\nefgh".into();
    app.preview.follow_bottom = false;
    app.preview.scroll = 1;

    let rows = preview_visible_plain_text_rows(&mut app, Rect::new(0, 0, 2, 2));

    assert_eq!(rows, vec!["cd".to_string(), "ef".to_string()]);
    assert!(app.preview.plain_cache.is_some());
}

#[test]
fn preview_selection_text_preserves_multiline_range() {
    let mut app = App::new();
    app.preview.source = PreviewSource::Tmux;
    app.preview.view = PreviewView::Plain;
    app.preview.pane_id = Some("%1".into());
    app.preview.content = "alpha\nbravo\ncharlie".into();

    let copied = extract_preview_selection_text(&mut app, Rect::new(0, 0, 20, 3), (2, 0), (2, 2));

    assert_eq!(copied.as_deref(), Some("pha\nbravo\ncha"));
}
