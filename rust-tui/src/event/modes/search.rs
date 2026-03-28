use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_search_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.mode = crate::app::state::Mode::Normal;
            app.is_searching = false;
            app.search_query.clear();
            app.invalidate_sidebar_visible_cache();
            app.sync_sidebar_selection();
            app.dirty = true;
        }
        KeyCode::Enter => {
            app.mode = crate::app::state::Mode::Normal;
            app.sync_sidebar_selection();
            app.dirty = true;
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.invalidate_sidebar_visible_cache();
            app.sync_sidebar_selection();
            app.invalidate_preview();
            app.dirty = true;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.invalidate_sidebar_visible_cache();
            app.sync_sidebar_selection();
            app.invalidate_preview();
            app.dirty = true;
        }
        _ => {}
    }
}
