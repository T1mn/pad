use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn handle_special_key(app: &mut App, key: KeyEvent) -> bool {
    if key.code == KeyCode::F(2) && app.open_thread_title_editor() {
        return true;
    }

    if key.code == KeyCode::Char('T') && app.open_thread_tags_editor() {
        return true;
    }

    if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.open_tree_in_home();
        return true;
    }

    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.mode = Mode::Search;
        app.is_searching = true;
        app.search_query.clear();
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();
        app.dirty = true;
        return true;
    }

    false
}
