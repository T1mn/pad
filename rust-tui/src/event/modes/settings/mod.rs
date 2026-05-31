mod appearance;
mod codex;
mod general;
mod list;
mod telegram;

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
        Some(SettingsDetailKind::AgentStyle) => {
            appearance::handle_agent_style_detail_mode(app, key)
        }
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
    fn settings_numeric_shortcuts_and_detail_search_shortcut_keep_current_behavior() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::List;

        handle_settings_mode(&mut app, KeyCode::Char('5'));
        assert_eq!(app.settings_selected, 4);

        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(matches!(app.settings_focus, SettingsFocus::Detail));

        app.settings_search = "stale".into();
        handle_settings_mode(&mut app, KeyCode::Char('/'));
        assert!(matches!(app.settings_focus, SettingsFocus::List));
        assert!(app.active_settings_detail.is_none());
        assert!(app.settings_searching);
        assert!(app.settings_search.is_empty());
    }

    #[test]
    fn settings_f1_closes_modal_from_detail_view() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::Detail;
        app.active_settings_detail = Some(SettingsDetailKind::Relay);
        app.settings_search = "relay".into();

        handle_settings_mode(&mut app, KeyCode::F(1));

        assert!(matches!(app.mode, Mode::Normal));
        assert!(!app.settings_open);
        assert!(matches!(app.settings_focus, SettingsFocus::List));
        assert!(app.active_settings_detail.is_none());
        assert!(!app.settings_searching);
        assert!(app.settings_search.is_empty());
    }

    #[test]
    fn theme_detail_escape_restores_preview_and_enter_applies_selection() {
        with_temp_home("theme-detail", || {
            let mut app = App::new();
            app.mode = Mode::Settings;
            app.settings_open = true;
            app.settings_selected = app
                .filtered_settings_items()
                .iter()
                .position(|(id, _, _, _, _)| *id == "theme")
                .expect("theme setting");
            app.enter_settings_detail();

            let original_theme = app.config.theme.clone();
            let preview_theme = App::available_themes()[1].0;

            handle_settings_mode(&mut app, KeyCode::Down);
            assert_eq!(app.theme.name, preview_theme);
            assert_eq!(app.config.theme, original_theme);

            handle_settings_mode(&mut app, KeyCode::Esc);
            assert!(matches!(app.settings_focus, SettingsFocus::List));
            assert!(app.active_settings_detail.is_none());
            assert_eq!(app.theme.name, original_theme);

            app.enter_settings_detail();
            handle_settings_mode(&mut app, KeyCode::Down);
            handle_settings_mode(&mut app, KeyCode::Enter);

            assert!(matches!(app.settings_focus, SettingsFocus::List));
            assert_eq!(app.config.theme, preview_theme);
            assert_eq!(app.theme.name, preview_theme);
        });
    }

    #[test]
    fn language_detail_escape_restores_locale_and_enter_persists_selection() {
        with_temp_home("language-detail", || {
            let mut app = App::new();
            app.mode = Mode::Settings;
            app.settings_open = true;
            app.settings_selected = app
                .filtered_settings_items()
                .iter()
                .position(|(id, _, _, _, _)| *id == "language")
                .expect("language setting");
            app.enter_settings_detail();

            let original_language = app.config.language.clone();
            let preview_locale = App::available_locales()[1];

            handle_settings_mode(&mut app, KeyCode::Down);
            assert_eq!(app.locale, preview_locale);
            assert_eq!(app.config.language, original_language);

            handle_settings_mode(&mut app, KeyCode::Esc);
            assert!(matches!(app.settings_focus, SettingsFocus::List));
            assert_eq!(app.locale.as_str(), original_language);

            app.enter_settings_detail();
            handle_settings_mode(&mut app, KeyCode::Down);
            handle_settings_mode(&mut app, KeyCode::Enter);

            assert!(matches!(app.settings_focus, SettingsFocus::List));
            assert_eq!(app.locale, preview_locale);
            assert_eq!(app.config.language, preview_locale.as_str());
        });
    }

    #[test]
    fn sound_settings_toggle_cycle_and_preview_work() {
        with_temp_home("sound-settings", || {
            let mut app = App::new();
            app.mode = Mode::Settings;
            app.settings_open = true;
            app.settings_focus = SettingsFocus::Detail;
            app.active_settings_detail = Some(SettingsDetailKind::Sound);
            app.sound_settings_selected = 0;
            crate::sound::with_test_sound_capture(|| {
                let _ = crate::sound::take_test_playbacks();

                handle_settings_mode(&mut app, KeyCode::Enter);
                assert!(!app.config.sound.enabled);

                handle_settings_mode(&mut app, KeyCode::Down);
                handle_settings_mode(&mut app, KeyCode::Enter);
                assert!(!app.config.sound.completion.enabled);

                handle_settings_mode(&mut app, KeyCode::Down);
                let original = app.config.sound.completion.preset.clone();
                handle_settings_mode(&mut app, KeyCode::Enter);
                assert_ne!(app.config.sound.completion.preset, original);

                let cycled = app.config.sound.completion.preset.clone();
                handle_settings_mode(&mut app, KeyCode::Char(' '));
                assert_eq!(app.config.sound.completion.preset, cycled);
                assert_eq!(
                    crate::sound::take_test_playbacks(),
                    vec![crate::sound::TestPlayback {
                        event: None,
                        preset: cycled,
                    }]
                );
            });
        });
    }

    #[test]
    fn telegram_settings_toggle_and_edit_fields_persist_without_leaving_detail() {
        with_temp_home("telegram-settings", || {
            let mut app = App::new();
            app.mode = Mode::Settings;
            app.settings_open = true;
            app.settings_focus = SettingsFocus::Detail;
            app.active_settings_detail = Some(SettingsDetailKind::Telegram);
            app.config.telegram.enabled = false;
            app.config.telegram.bot_token.clear();
            app.config.telegram.chat_id.clear();

            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(app.config.telegram.enabled);

            handle_settings_mode(&mut app, KeyCode::Char('j'));
            assert_eq!(app.telegram_selected_field, 1);
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(app.telegram_editing);

            for ch in "bot-token".chars() {
                handle_settings_mode(&mut app, KeyCode::Char(ch));
            }
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(!app.telegram_editing);
            assert_eq!(app.config.telegram.bot_token, "bot-token");

            handle_settings_mode(&mut app, KeyCode::Char('j'));
            assert_eq!(app.telegram_selected_field, 2);
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(app.telegram_editing);

            for ch in "chat-id".chars() {
                handle_settings_mode(&mut app, KeyCode::Char(ch));
            }
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(!app.telegram_editing);
            assert_eq!(app.config.telegram.chat_id, "chat-id");
            assert!(matches!(app.settings_focus, SettingsFocus::Detail));
        });
    }

    #[test]
    fn telegram_settings_edit_escape_discards_buffer_and_r_keeps_detail_open() {
        with_temp_home("telegram-settings-escape", || {
            let mut app = App::new();
            app.mode = Mode::Settings;
            app.settings_open = true;
            app.settings_focus = SettingsFocus::Detail;
            app.active_settings_detail = Some(SettingsDetailKind::Telegram);
            app.config.telegram.enabled = false;
            app.config.telegram.bot_token = "seed".into();

            app.telegram_selected_field = 1;
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert!(app.telegram_editing);
            assert_eq!(app.telegram_edit_buffer, "seed");

            handle_settings_mode(&mut app, KeyCode::Char('x'));
            assert_eq!(app.telegram_edit_buffer, "seedx");

            handle_settings_mode(&mut app, KeyCode::Esc);
            assert!(!app.telegram_editing);
            assert!(app.telegram_edit_buffer.is_empty());
            assert_eq!(app.config.telegram.bot_token, "seed");

            handle_settings_mode(&mut app, KeyCode::Char('r'));
            assert!(matches!(app.settings_focus, SettingsFocus::Detail));
            assert!(matches!(
                app.active_settings_detail,
                Some(SettingsDetailKind::Telegram)
            ));
        });
    }
}
