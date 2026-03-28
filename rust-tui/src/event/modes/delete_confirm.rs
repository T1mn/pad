use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_delete_confirm_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(panel) = app.sidebar.delete_target.take() {
                app.delete_panel(&panel);
            }
            app.mode = Mode::Normal;
            app.dirty = true;
        }
        _ => {
            app.sidebar.delete_target = None;
            app.mode = Mode::Normal;
            app.dirty = true;
        }
    }
}
