use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_file_preview_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Tree;
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.preview.file_preview_scroll = app.preview.file_preview_scroll.saturating_add(1);
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.preview.file_preview_scroll = app.preview.file_preview_scroll.saturating_sub(1);
            app.dirty = true;
        }
        KeyCode::Char('J') => {
            app.preview.file_preview_scroll = app.preview.file_preview_scroll.saturating_add(3);
            app.dirty = true;
        }
        KeyCode::Char('K') => {
            app.preview.file_preview_scroll = app.preview.file_preview_scroll.saturating_sub(3);
            app.dirty = true;
        }
        KeyCode::PageDown => {
            app.preview.file_preview_scroll = app.preview.file_preview_scroll.saturating_add(20);
            app.dirty = true;
        }
        KeyCode::PageUp => {
            app.preview.file_preview_scroll = app.preview.file_preview_scroll.saturating_sub(20);
            app.dirty = true;
        }
        KeyCode::Home => {
            app.preview.file_preview_scroll = 0;
            app.dirty = true;
        }
        KeyCode::End => {
            app.preview.file_preview_scroll = u16::MAX;
            app.dirty = true;
        }
        _ => {}
    }
}
