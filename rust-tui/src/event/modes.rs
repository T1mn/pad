mod agent_launcher;
mod delete_confirm {
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
}
mod file_preview {
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
                app.preview.file_preview_scroll =
                    app.preview.file_preview_scroll.saturating_add(20);
                app.dirty = true;
            }
            KeyCode::PageUp => {
                app.preview.file_preview_scroll =
                    app.preview.file_preview_scroll.saturating_sub(20);
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
}
mod fuzzy_picker {
    use crate::app::App;
    use crossterm::event::KeyEvent;

    pub(crate) fn handle_fuzzy_picker_mode(app: &mut App, key: KeyEvent) {
        if let Some(ref mut picker) = app.fuzzy_picker {
            match picker.handle_input(key) {
                None => {
                    app.dirty = true;
                }
                Some(None) => {
                    app.close_fuzzy_picker();
                }
                Some(Some(path)) => {
                    app.fuzzy_picker = None;
                    app.open_agent_launcher(std::path::PathBuf::from(path));
                }
            }
        }
    }
}
mod help {
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
}
mod notification_inbox;
mod relay_settings;
mod search {
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
}
mod settings;
mod telegram;
mod thread_action_confirm;
mod tree;
mod tree_search {
    use crate::app::state::Mode;
    use crate::app::App;
    use crossterm::event::KeyCode;

    pub(crate) fn handle_tree_search_mode(app: &mut App, key: KeyCode) {
        if let Some(ref mut tree) = app.sidebar.file_tree {
            match key {
                KeyCode::Esc => {
                    tree.cancel_search();
                    app.mode = Mode::Tree;
                    app.update_file_preview();
                    app.dirty = true;
                }
                KeyCode::Enter => {
                    tree.cancel_search();
                    app.mode = Mode::Tree;
                    app.update_file_preview();
                    app.dirty = true;
                }
                KeyCode::Char(c) => {
                    tree.search_input(c);
                    app.update_file_preview();
                    app.dirty = true;
                }
                KeyCode::Backspace => {
                    tree.search_backspace();
                    app.update_file_preview();
                    app.dirty = true;
                }
                _ => {}
            }
        }
    }
}

pub(crate) use agent_launcher::handle_agent_launcher_mode;
pub(crate) use delete_confirm::handle_delete_confirm_mode;
pub(crate) use file_preview::handle_file_preview_mode;
pub(crate) use fuzzy_picker::handle_fuzzy_picker_mode;
pub(crate) use help::handle_help_mode;
pub(crate) use notification_inbox::handle_notification_inbox_mode;
pub(crate) use relay_settings::handle_relay_settings_mode;
pub(crate) use search::handle_search_mode;
pub(crate) use settings::handle_settings_mode;
pub(crate) use telegram::handle_telegram_settings_mode;
pub(crate) use thread_action_confirm::handle_thread_action_confirm_mode;
pub(crate) use tree::handle_tree_mode;
pub(crate) use tree_search::handle_tree_search_mode;
