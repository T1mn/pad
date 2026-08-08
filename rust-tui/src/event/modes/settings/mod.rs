mod appearance {
    use crate::app::state::SettingsFocus;
    use crate::app::App;
    use crossterm::event::KeyCode;

    pub(super) fn handle_theme_detail_mode(app: &mut App, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                app.leave_settings_detail();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = App::available_themes().len().saturating_sub(1);
                if app.theme_selected < max {
                    app.theme_selected += 1;
                }
                let themes = App::available_themes();
                if let Some((name, _)) = themes.get(app.theme_selected) {
                    app.preview_theme(name);
                }
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.theme_selected > 0 {
                    app.theme_selected -= 1;
                }
                let themes = App::available_themes();
                if let Some((name, _)) = themes.get(app.theme_selected) {
                    app.preview_theme(name);
                }
                app.dirty = true;
            }
            KeyCode::Char(c @ '1'..='9') => {
                let idx = (c as usize) - ('1' as usize);
                app.theme_selected = idx.min(App::available_themes().len().saturating_sub(1));
                let themes = App::available_themes();
                if let Some((name, _)) = themes.get(app.theme_selected) {
                    app.preview_theme(name);
                }
                app.dirty = true;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let themes = App::available_themes();
                if let Some((name, _)) = themes.get(app.theme_selected) {
                    app.apply_theme(name);
                    app.settings_focus = SettingsFocus::List;
                    app.dirty = true;
                }
            }
            _ => {}
        }
        true
    }

    pub(super) fn handle_language_detail_mode(app: &mut App, key: KeyCode) -> bool {
        let locales = App::available_locales();
        match key {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                app.leave_settings_detail();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = locales.len().saturating_sub(1);
                if app.language_selected < max {
                    app.language_selected += 1;
                }
                if let Some(locale) = locales.get(app.language_selected) {
                    app.locale = *locale;
                }
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.language_selected > 0 {
                    app.language_selected -= 1;
                }
                if let Some(locale) = locales.get(app.language_selected) {
                    app.locale = *locale;
                }
                app.dirty = true;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(locale) = locales.get(app.language_selected) {
                    app.locale = *locale;
                    app.config.language = locale.as_str().to_string();
                    app.save_config();
                }
                app.settings_focus = SettingsFocus::List;
                app.dirty = true;
            }
            _ => {}
        }
        true
    }
}
mod codex;
mod general;
mod list {
    use crate::app::state::SettingsFocus;
    use crate::app::App;
    use crossterm::event::KeyCode;

    pub(super) fn handle_settings_search_key(app: &mut App, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                app.settings_searching = false;
                app.settings_search.clear();
                app.dirty = true;
            }
            KeyCode::Enter => {
                app.settings_searching = false;
                if !app.filtered_settings_items().is_empty() {
                    app.enter_settings_detail();
                } else {
                    app.dirty = true;
                }
            }
            KeyCode::Down => {
                move_settings_selection_down(app);
                app.dirty = true;
            }
            KeyCode::Up => {
                move_settings_selection_up(app);
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                app.settings_search.push(c);
                app.settings_selected = 0;
                app.dirty = true;
            }
            KeyCode::Backspace => {
                app.settings_search.pop();
                app.settings_selected = 0;
                app.dirty = true;
            }
            _ => {}
        }
        true
    }

    pub(super) fn handle_settings_list_key(app: &mut App, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::F(1) => {
                app.close_settings();
            }
            KeyCode::Char('/') => {
                app.settings_focus = SettingsFocus::List;
                app.settings_searching = true;
                app.settings_search.clear();
                app.settings_selected = 0;
                app.dirty = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                move_settings_selection_down(app);
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                move_settings_selection_up(app);
                app.dirty = true;
            }
            KeyCode::Char('1') => set_settings_selection(app, 0),
            KeyCode::Char('2') => set_settings_selection(app, 1),
            KeyCode::Char('3') => set_settings_selection(app, 2),
            KeyCode::Char('4') => set_settings_selection(app, 3),
            KeyCode::Char('5') => set_settings_selection(app, 4),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                app.enter_settings_detail();
            }
            _ => {}
        }
    }

    fn set_settings_selection(app: &mut App, index: usize) {
        app.settings_selected = index.min(app.filtered_settings_items().len().saturating_sub(1));
        app.dirty = true;
    }

    pub(super) fn move_settings_selection_down(app: &mut App) {
        let max = app.filtered_settings_items().len().saturating_sub(1);
        if app.settings_selected < max {
            app.settings_selected += 1;
        }
    }

    pub(super) fn move_settings_selection_up(app: &mut App) {
        if app.settings_selected > 0 {
            app.settings_selected -= 1;
        }
    }
}
mod telegram {
    use crate::app::App;
    use crate::log_debug;
    use crate::telegram;
    use crossterm::event::KeyCode;

    pub(super) fn handle_telegram_detail_mode(app: &mut App, key: KeyCode) -> bool {
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
            return true;
        }

        match key {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                app.leave_settings_detail();
            }
            KeyCode::Char('r') => restart_telegram_daemon(app),
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
                    3 => restart_telegram_daemon(app),
                    _ => {}
                }
                app.dirty = true;
            }
            _ => {}
        }
        true
    }

    fn restart_telegram_daemon(app: &mut App) {
        if let Err(err) = telegram::restart_daemon(&app.config) {
            log_debug!("telegram: restart failed from settings: {}", err);
        }
        app.dirty = true;
    }
}

use super::relay_settings::{handle_relay_key, RelayHost};
use crate::app::state::{SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_settings_mode(app: &mut App, key: KeyCode) {
    if app.settings_searching {
        let _ = list::handle_settings_search_key(app, key);
        return;
    }

    if app.settings_focus == SettingsFocus::Detail && handle_settings_detail_mode(app, key) {
        return;
    }

    list::handle_settings_list_key(app, key);
}

fn handle_settings_detail_mode(app: &mut App, key: KeyCode) -> bool {
    if key == KeyCode::F(1) {
        app.close_settings();
        return true;
    }
    if key == KeyCode::Char('/') {
        app.leave_settings_detail();
        app.settings_searching = true;
        app.settings_search.clear();
        app.dirty = true;
        return true;
    }
    match app.current_settings_detail_kind() {
        Some(SettingsDetailKind::Theme) => appearance::handle_theme_detail_mode(app, key),
        Some(SettingsDetailKind::Language) => appearance::handle_language_detail_mode(app, key),
        Some(SettingsDetailKind::CodexSettings) => {
            codex::handle_codex_settings_detail_mode(app, key)
        }
        Some(SettingsDetailKind::ClaudeFullAccess) => {
            general::handle_claude_full_access_detail_mode(app, key)
        }
        Some(SettingsDetailKind::Sound) => general::handle_sound_detail_mode(app, key),
        Some(SettingsDetailKind::Relay) => handle_relay_detail_mode(app, key),
        Some(SettingsDetailKind::Telegram) => telegram::handle_telegram_detail_mode(app, key),
        Some(SettingsDetailKind::AutoRefresh) => general::handle_auto_refresh_detail_mode(app, key),
        Some(SettingsDetailKind::PreviewMode) => general::handle_preview_mode_detail_mode(app, key),
        Some(SettingsDetailKind::DisplayMode) => general::handle_display_mode_detail_mode(app, key),
        Some(SettingsDetailKind::Trash) => general::handle_trash_detail_mode(app, key),
        Some(SettingsDetailKind::Version) => {
            match key {
                KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
                _ => {}
            }
            true
        }
        None => false,
    }
}

fn handle_relay_detail_mode(app: &mut App, key: KeyCode) -> bool {
    handle_relay_key(app, key, RelayHost::Settings)
}

#[cfg(test)]
mod tests;
