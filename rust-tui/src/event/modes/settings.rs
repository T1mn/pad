use super::relay_settings::{handle_relay_key, RelayHost};
use crate::app::state::{SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crate::log_debug;
use crate::relay;
use crate::telegram;
use crossterm::event::KeyCode;

pub(crate) fn handle_settings_mode(app: &mut App, key: KeyCode) {
    if app.settings_searching {
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
        return;
    }

    if app.settings_focus == SettingsFocus::Detail && handle_settings_detail_mode(app, key) {
        return;
    }

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
        KeyCode::Char('1') => {
            app.settings_selected = 0;
            app.dirty = true;
        }
        KeyCode::Char('2') => {
            app.settings_selected = 1.min(app.filtered_settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Char('3') => {
            app.settings_selected = 2.min(app.filtered_settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Char('4') => {
            app.settings_selected = 3.min(app.filtered_settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Char('5') => {
            app.settings_selected = 4.min(app.filtered_settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            app.enter_settings_detail();
        }
        _ => {}
    }
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
        Some(SettingsDetailKind::Theme) => handle_theme_detail_mode(app, key),
        Some(SettingsDetailKind::Language) => handle_language_detail_mode(app, key),
        Some(SettingsDetailKind::AgentStyle) => handle_agent_style_detail_mode(app, key),
        Some(SettingsDetailKind::CodexSettings) => handle_codex_settings_detail_mode(app, key),
        Some(SettingsDetailKind::ClaudeFullAccess) => {
            handle_claude_full_access_detail_mode(app, key)
        }
        Some(SettingsDetailKind::Relay) => handle_relay_detail_mode(app, key),
        Some(SettingsDetailKind::Telegram) => handle_telegram_detail_mode(app, key),
        Some(SettingsDetailKind::AutoRefresh) => handle_auto_refresh_detail_mode(app, key),
        Some(SettingsDetailKind::PreviewMode) => handle_preview_mode_detail_mode(app, key),
        Some(SettingsDetailKind::DisplayMode) => handle_display_mode_detail_mode(app, key),
        Some(SettingsDetailKind::Trash) => handle_trash_detail_mode(app, key),
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

fn handle_trash_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => app.open_trash_threads_view(),
        _ => {}
    }
    true
}

fn handle_theme_detail_mode(app: &mut App, key: KeyCode) -> bool {
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

fn handle_language_detail_mode(app: &mut App, key: KeyCode) -> bool {
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
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.language_selected > 0 {
                app.language_selected -= 1;
            }
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
                app.config.language = l.as_str().to_string();
                app.config.save();
            }
            app.settings_focus = SettingsFocus::List;
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_agent_style_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.leave_settings_detail();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.agent_style_selected < 1 {
                app.agent_style_selected += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.agent_style_selected > 0 {
                app.agent_style_selected -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            match app.agent_style_selected {
                0 => {
                    app.config.desired_agent_style.zoom =
                        if app.config.desired_agent_style.zoom == "auto" {
                            "keep".to_string()
                        } else {
                            "auto".to_string()
                        };
                }
                1 => {
                    app.config.desired_agent_style.status =
                        match app.config.desired_agent_style.status.as_str() {
                            "show" => "hide".to_string(),
                            "hide" => "keep".to_string(),
                            _ => "show".to_string(),
                        };
                }
                _ => {}
            }
            app.config.save();
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_auto_refresh_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.config.auto_refresh = !app.config.auto_refresh;
            app.config.save();
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_codex_settings_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Char('j') | KeyCode::Down => {
            if app.codex_settings_selected < 4 {
                app.codex_settings_selected += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.codex_settings_selected > 0 {
                app.codex_settings_selected -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            match app.codex_settings_selected {
                0 => {
                    app.config.agent_permissions.codex_auto_full_access =
                        !app.config.agent_permissions.codex_auto_full_access;
                }
                1 => {
                    app.config.codex.fast_mode = !app.config.codex.fast_mode;
                }
                2 => {
                    app.config.codex.multi_agent = !app.config.codex.multi_agent;
                }
                3 => {
                    app.config.codex.web_search = match app.config.codex.web_search.as_str() {
                        "cached" => "live".to_string(),
                        "live" => "disabled".to_string(),
                        "disabled" => "default".to_string(),
                        _ => "cached".to_string(),
                    };
                }
                4 => {
                    app.config.codex.title_summary = !app.config.codex.title_summary;
                }
                _ => {}
            }
            app.config.save();
            relay::apply_runtime_configs(
                &app.config.agents,
                &app.config.agent_permissions,
                &app.config.codex,
            );
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_claude_full_access_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.config.agent_permissions.claude_auto_full_access =
                !app.config.agent_permissions.claude_auto_full_access;
            app.config.save();
            relay::apply_runtime_configs(
                &app.config.agents,
                &app.config.agent_permissions,
                &app.config.codex,
            );
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_preview_mode_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.config.preview.mode = match app.config.preview.mode.as_str() {
                "auto" => "tmux".to_string(),
                "tmux" => "session".to_string(),
                _ => "auto".to_string(),
            };
            app.config.save();
            app.invalidate_preview();
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_display_mode_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => {
            let next_scope = if app.config.display.session_scope == "live" {
                "all"
            } else {
                "live"
            };
            app.apply_display_session_scope(next_scope, true);
        }
        _ => {}
    }
    true
}

fn handle_relay_detail_mode(app: &mut App, key: KeyCode) -> bool {
    handle_relay_key(app, key, RelayHost::Settings)
}

fn handle_telegram_detail_mode(app: &mut App, key: KeyCode) -> bool {
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
                        restart_needed = app.config.telegram.bot_token != app.telegram_edit_buffer;
                        app.config.telegram.bot_token = app.telegram_edit_buffer.clone();
                    }
                    2 => app.config.telegram.chat_id = app.telegram_edit_buffer.clone(),
                    _ => {}
                }
                app.config.save();
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
                    app.config.save();
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

fn move_settings_selection_down(app: &mut App) {
    let max = app.filtered_settings_items().len().saturating_sub(1);
    if app.settings_selected < max {
        app.settings_selected += 1;
    }
}

fn move_settings_selection_up(app: &mut App) {
    if app.settings_selected > 0 {
        app.settings_selected -= 1;
    }
}

fn restart_telegram_daemon(app: &mut App) {
    if let Err(err) = telegram::restart_daemon(&app.config) {
        log_debug!("telegram: restart failed from settings: {}", err);
    }
    app.dirty = true;
}

#[cfg(test)]
mod tests {
    use super::handle_settings_mode;
    use crate::app::state::{Mode, SettingsDetailKind, SettingsFocus};
    use crate::app::App;
    use crossterm::event::KeyCode;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
        let _guard = crate::test_support::home_env_lock()
            .lock()
            .expect("lock settings tests");
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let home = std::env::temp_dir().join(format!("pad-settings-{name}-{stamp}"));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("create temp home");

        let prev_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &home);

        let result = f();

        if let Some(prev) = prev_home {
            std::env::set_var("HOME", prev);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = std::fs::remove_dir_all(&home);
        result
    }

    #[test]
    fn settings_search_enter_opens_first_match_directly() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::List;
        app.settings_searching = true;
        app.settings_search = "relay".into();
        app.settings_selected = 0;

        handle_settings_mode(&mut app, KeyCode::Enter);

        assert!(!app.settings_searching);
        assert!(matches!(app.settings_focus, SettingsFocus::Detail));
        assert!(matches!(
            app.current_settings_detail_kind(),
            Some(crate::app::state::SettingsDetailKind::Relay)
        ));
    }

    #[test]
    fn settings_search_enter_with_no_match_stays_in_list() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::List;
        app.settings_searching = true;
        app.settings_search = "no-such-setting".into();
        app.settings_selected = 0;

        handle_settings_mode(&mut app, KeyCode::Enter);

        assert!(!app.settings_searching);
        assert!(matches!(app.settings_focus, SettingsFocus::List));
        assert!(app.current_settings_detail_kind().is_none());
    }

    #[test]
    fn settings_search_allows_arrow_navigation_without_query() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::List;
        app.settings_searching = true;
        app.settings_search.clear();
        app.settings_selected = 0;

        handle_settings_mode(&mut app, KeyCode::Down);
        assert_eq!(app.settings_selected, 1);

        handle_settings_mode(&mut app, KeyCode::Up);
        assert_eq!(app.settings_selected, 0);
    }

    #[test]
    fn settings_search_allows_arrow_navigation_with_filtered_results() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::List;
        app.settings_searching = true;
        app.settings_search = "settings".into();
        app.settings_selected = 0;

        assert!(app.filtered_settings_items().len() >= 2);

        handle_settings_mode(&mut app, KeyCode::Down);
        assert_eq!(app.settings_selected, 1);

        handle_settings_mode(&mut app, KeyCode::Up);
        assert_eq!(app.settings_selected, 0);
    }

    #[test]
    fn codex_settings_enter_toggles_all_switches_and_cycles_web_search() {
        with_temp_home("codex-settings-toggle", || {
            let mut app = App::new();
            app.mode = Mode::Settings;
            app.settings_open = true;
            app.settings_focus = SettingsFocus::Detail;
            app.active_settings_detail = Some(SettingsDetailKind::CodexSettings);
            app.config.agent_permissions.codex_auto_full_access = false;
            app.config.codex.fast_mode = false;
            app.config.codex.multi_agent = false;
            app.config.codex.web_search = "default".into();
            app.config.codex.title_summary = false;

            app.codex_settings_selected = 0;
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(app.config.agent_permissions.codex_auto_full_access);

            handle_settings_mode(&mut app, KeyCode::Down);
            assert_eq!(app.codex_settings_selected, 1);
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(app.config.codex.fast_mode);

            handle_settings_mode(&mut app, KeyCode::Down);
            assert_eq!(app.codex_settings_selected, 2);
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(app.config.codex.multi_agent);

            handle_settings_mode(&mut app, KeyCode::Down);
            assert_eq!(app.codex_settings_selected, 3);
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert_eq!(app.config.codex.web_search, "cached");
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert_eq!(app.config.codex.web_search, "live");
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert_eq!(app.config.codex.web_search, "disabled");
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert_eq!(app.config.codex.web_search, "default");

            handle_settings_mode(&mut app, KeyCode::Down);
            assert_eq!(app.codex_settings_selected, 4);
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(app.config.codex.title_summary);
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(!app.config.codex.title_summary);
        });
    }

    #[test]
    fn codex_settings_navigation_is_bounded() {
        with_temp_home("codex-settings-nav", || {
            let mut app = App::new();
            app.mode = Mode::Settings;
            app.settings_open = true;
            app.settings_focus = SettingsFocus::Detail;
            app.active_settings_detail = Some(SettingsDetailKind::CodexSettings);
            app.codex_settings_selected = 0;

            handle_settings_mode(&mut app, KeyCode::Up);
            assert_eq!(app.codex_settings_selected, 0);

            for _ in 0..10 {
                handle_settings_mode(&mut app, KeyCode::Down);
            }
            assert_eq!(app.codex_settings_selected, 4);

            handle_settings_mode(&mut app, KeyCode::Esc);
            assert!(matches!(app.settings_focus, SettingsFocus::List));
            assert!(app.active_settings_detail.is_none());
        });
    }
}
