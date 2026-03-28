use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_help_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.mode = Mode::Normal;
            app.dirty = true;
        }
        _ => {}
    }
}
