use crate::app::App;
use crossterm::event::KeyEvent;

mod jump {
    use crate::app::App;
    use crossterm::event::{KeyCode, KeyEvent};

    pub(super) fn handle_numeric_jump(app: &mut App, key: KeyEvent) -> bool {
        let Some(index) = numeric_jump_index(key.code) else {
            return false;
        };
        app.jump_to(index);
        true
    }

    fn numeric_jump_index(code: KeyCode) -> Option<usize> {
        match code {
            KeyCode::Char('1') => Some(0),
            KeyCode::Char('2') => Some(1),
            KeyCode::Char('3') => Some(2),
            KeyCode::Char('4') => Some(3),
            KeyCode::Char('5') => Some(4),
            KeyCode::Char('6') => Some(5),
            KeyCode::Char('7') => Some(6),
            KeyCode::Char('8') => Some(7),
            KeyCode::Char('9') => Some(8),
            _ => None,
        }
    }
}
mod opencode;
mod primary;
mod special {
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

        if key.code == KeyCode::Char('L') {
            let current_width = crate::ui::panel_list::preferred_panel_width(app);
            app.widen_agent_panel_width(current_width);
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
}

pub(super) fn handle_global_key(app: &mut App, key: KeyEvent) -> bool {
    special::handle_special_key(app, key)
        || primary::handle_primary_key(app, key)
        || opencode::handle_opencode_key(app, key)
        || jump::handle_numeric_jump(app, key)
}
