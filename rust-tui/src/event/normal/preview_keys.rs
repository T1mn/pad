use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle_preview_key(app: &mut App, key: KeyEvent) -> bool {
    if !app.preview_is_focused() {
        return false;
    }

    match key.code {
        KeyCode::Esc => {
            app.step_back_preview_focus();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.scroll_preview_by(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.scroll_preview_by(-1);
        }
        KeyCode::Char('J') => {
            if app.has_session_preview_turns() {
                app.select_next_preview_turn();
            } else {
                app.scroll_preview_by(10);
            }
        }
        KeyCode::Char('K') => {
            if app.has_session_preview_turns() {
                app.select_previous_preview_turn();
            } else {
                app.scroll_preview_by(-10);
            }
        }
        KeyCode::PageDown => {
            app.scroll_preview_by(20);
        }
        KeyCode::PageUp => {
            app.scroll_preview_by(-20);
        }
        KeyCode::Home => {
            app.scroll_preview_to_top();
        }
        KeyCode::End => {
            app.scroll_preview_to_bottom();
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let _ = app.toggle_preview_turn_expanded();
        }
        _ => {}
    }

    true
}
