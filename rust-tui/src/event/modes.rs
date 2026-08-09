pub(crate) mod agent_launcher;
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
pub(crate) mod notification_inbox;
pub(crate) mod relay_settings;
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
pub(crate) mod settings;
mod telegram {
    use crate::app::App;
    use crate::log_debug;
    use crate::telegram;
    use crossterm::event::KeyCode;

    pub(crate) fn handle_telegram_settings_mode(app: &mut App, key: KeyCode) {
        if app.telegram_editing {
            match key {
                KeyCode::Esc => {
                    app.telegram_editing = false;
                    app.telegram_edit_buffer.clear();
                    app.dirty = true;
                }
                KeyCode::Enter => {
                    let mut restart_needed = false;
                    match app.telegram_selected_field {
                        1 => {
                            restart_needed =
                                app.config.telegram.bot_token != app.telegram_edit_buffer;
                            app.config.telegram.bot_token = app.telegram_edit_buffer.clone();
                        }
                        2 => app.config.telegram.chat_id = app.telegram_edit_buffer.clone(),
                        _ => {}
                    }
                    app.save_config();
                    let daemon_result = if restart_needed {
                        telegram::restart_daemon(&app.config)
                    } else {
                        telegram::sync_daemon(&app.config)
                    };
                    if let Err(err) = daemon_result {
                        log_debug!("telegram: daemon sync failed after settings save: {}", err);
                    }
                    app.telegram_editing = false;
                    app.telegram_edit_buffer.clear();
                    app.dirty = true;
                }
                KeyCode::Backspace => {
                    app.telegram_edit_buffer.pop();
                    app.dirty = true;
                }
                KeyCode::Char(c) => {
                    app.telegram_edit_buffer.push(c);
                    app.dirty = true;
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Esc => {
                app.mode = crate::app::state::Mode::Settings;
                app.dirty = true;
            }
            KeyCode::Char('r') => {
                restart_telegram_daemon(app);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if app.telegram_selected_field < 3 {
                    app.telegram_selected_field += 1;
                }
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.telegram_selected_field > 0 {
                    app.telegram_selected_field -= 1;
                }
                app.dirty = true;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                match app.telegram_selected_field {
                    0 => {
                        app.config.telegram.enabled = !app.config.telegram.enabled;
                        app.save_config();
                        if let Err(err) = telegram::sync_daemon(&app.config) {
                            log_debug!("telegram: daemon sync failed after toggle: {}", err);
                        }
                    }
                    1 => {
                        app.telegram_edit_buffer = app.config.telegram.bot_token.clone();
                        app.telegram_editing = true;
                    }
                    2 => {
                        app.telegram_edit_buffer = app.config.telegram.chat_id.clone();
                        app.telegram_editing = true;
                    }
                    3 => {
                        restart_telegram_daemon(app);
                    }
                    _ => {}
                }
                app.dirty = true;
            }
            _ => {}
        }
    }

    fn restart_telegram_daemon(app: &mut App) {
        if let Err(err) = telegram::restart_daemon(&app.config) {
            log_debug!("telegram: restart failed from settings: {}", err);
        }
        app.dirty = true;
    }
}
pub(crate) mod thread_action_confirm;
mod tree {
    use crate::app::state::Mode;
    use crate::app::App;
    use crate::log_debug;
    use crossterm::event::KeyCode;

    pub(crate) fn handle_tree_mode(app: &mut App, key: KeyCode) {
        if let Some(ref mut tree) = app.sidebar.file_tree {
            log_debug!(
                "tree_mode key={:?} path={} selected={:?}",
                key,
                tree.current_path.display(),
                tree.state.selected()
            );
            match key {
                KeyCode::Esc => {
                    app.close_tree();
                    app.dirty = true;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    tree.next();
                    app.update_file_preview();
                    app.dirty = true;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    tree.previous();
                    app.update_file_preview();
                    app.dirty = true;
                }
                KeyCode::Char(' ') => {
                    tree.toggle();
                    app.update_file_preview();
                    app.dirty = true;
                }
                KeyCode::Enter => {
                    let entry_name = tree.selected().map(|e| e.name.clone()).unwrap_or_default();
                    log_debug!("tree_mode enter: entry={}", entry_name);
                    let selected_is_dir = tree.selected().map(|e| e.is_dir).unwrap_or(false);
                    if selected_is_dir {
                        tree.enter();
                        app.update_file_preview();
                    } else {
                        app.mode = Mode::FilePreview;
                        app.preview.file_preview_scroll = 0;
                    }
                    app.dirty = true;
                }
                KeyCode::Backspace => {
                    tree.go_up();
                    app.update_file_preview();
                    app.dirty = true;
                }
                KeyCode::Char('/') => {
                    app.mode = Mode::TreeSearch;
                    tree.start_search();
                    app.dirty = true;
                }
                KeyCode::Char('c') => {
                    let target_path = tree.selected().filter(|e| e.is_dir).map(|e| e.path.clone());
                    if let Some(path) = target_path {
                        log_debug!("tree_mode: open agent launcher at {}", path.display());
                        app.open_agent_launcher(path);
                    }
                }
                KeyCode::Char('T') => {
                    app.open_tree_in_home();
                }
                KeyCode::Char('t') => {
                    app.toggle_tree();
                }
                KeyCode::Char('J') => {
                    app.preview.file_preview_scroll =
                        app.preview.file_preview_scroll.saturating_add(3);
                    app.dirty = true;
                }
                KeyCode::Char('K') => {
                    app.preview.file_preview_scroll =
                        app.preview.file_preview_scroll.saturating_sub(3);
                    app.dirty = true;
                }
                KeyCode::PageDown => {
                    app.preview.file_preview_scroll =
                        app.preview.file_preview_scroll.saturating_add(10);
                    app.dirty = true;
                }
                KeyCode::PageUp => {
                    app.preview.file_preview_scroll =
                        app.preview.file_preview_scroll.saturating_sub(10);
                    app.dirty = true;
                }
                _ => {}
            }
        }
    }
}
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
