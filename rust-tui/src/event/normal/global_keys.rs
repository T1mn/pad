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
mod opencode {
    use crate::app::App;
    use crossterm::event::{KeyCode, KeyEvent};

    pub(super) fn handle_opencode_key(app: &mut App, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('E') => {
                let _ = app.export_selected_opencode_thread();
                true
            }
            KeyCode::Char('S') => {
                let _ = app.export_sanitized_selected_opencode_thread();
                true
            }
            KeyCode::Char('I') => {
                let _ = app.import_opencode_thread_from_clipboard();
                true
            }
            KeyCode::Char('O') => {
                let _ = app.export_selected_opencode_stats();
                true
            }
            KeyCode::Char('P') => {
                let _ = app.export_opencode_diagnostics();
                true
            }
            KeyCode::Char('Y') => {
                let _ = app.attach_opencode_from_clipboard();
                true
            }
            KeyCode::Char('W') => {
                let _ = app.open_opencode_web_for_selected_thread();
                true
            }
            KeyCode::Char('X') => {
                let _ = app.run_opencode_prompt_from_clipboard();
                true
            }
            KeyCode::Char('B') => {
                let _ = app.serve_opencode_for_selected_thread();
                true
            }
            KeyCode::Char('G') => {
                let _ = app.open_opencode_pr_from_clipboard();
                true
            }
            KeyCode::Char('M') => {
                let _ = app.install_opencode_plugin_from_clipboard();
                true
            }
            KeyCode::Char('H') => {
                let _ = app.install_opencode_github_agent();
                true
            }
            _ => false,
        }
    }
}
mod layout {
    use crate::app::App;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum SidebarLayoutAction {
        Toggle,
        Narrow,
        Widen,
    }

    pub(in crate::event::normal) fn handle_sidebar_layout_key(
        app: &mut App,
        key: KeyEvent,
    ) -> bool {
        let Some(action) = sidebar_layout_action(key) else {
            return false;
        };
        match action {
            SidebarLayoutAction::Toggle => app.toggle_sidebar_collapsed(),
            SidebarLayoutAction::Narrow | SidebarLayoutAction::Widen => {
                app.expand_sidebar();
                let current_width = crate::ui::panel_list::preferred_panel_width(app);
                if action == SidebarLayoutAction::Narrow {
                    app.narrow_agent_panel_width(current_width);
                } else {
                    app.widen_agent_panel_width(current_width);
                }
            }
        }
        true
    }

    fn sidebar_layout_action(key: KeyEvent) -> Option<SidebarLayoutAction> {
        match key.code {
            // F13-F15 are private host-bridge keys used by the conditional
            // Kitty mappings documented in README. They avoid stealing
            // Ctrl+B/H/L from shells and terminal applications.
            KeyCode::F(13) => Some(SidebarLayoutAction::Toggle),
            KeyCode::F(14) => Some(SidebarLayoutAction::Narrow),
            KeyCode::F(15) => Some(SidebarLayoutAction::Widen),
            KeyCode::Char('b' | 'B') if key.modifiers.contains(KeyModifiers::SUPER) => {
                Some(SidebarLayoutAction::Toggle)
            }
            KeyCode::Char('h' | 'H')
                if key.modifiers.contains(KeyModifiers::SUPER)
                    && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                Some(SidebarLayoutAction::Narrow)
            }
            KeyCode::Char('l' | 'L')
                if key.modifiers.contains(KeyModifiers::SUPER)
                    && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                Some(SidebarLayoutAction::Widen)
            }
            _ => None,
        }
    }
}
pub(crate) mod primary;
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

pub(super) use layout::handle_sidebar_layout_key;
