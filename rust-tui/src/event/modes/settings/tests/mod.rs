pub(crate) mod appearance {
    use super::super::handle_settings_mode;
    use super::support::with_temp_home;
    use crate::app::state::{Mode, SettingsFocus};
    use crate::app::App;
    use crossterm::event::KeyCode;

    pub(crate) fn theme_detail_escape_restores_preview_and_enter_applies_selection() {
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

    pub(crate) fn language_detail_escape_restores_locale_and_enter_persists_selection() {
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
}
pub(crate) mod search {
    use super::super::handle_settings_mode;
    use crate::app::state::{
        CodexSettingsView, Mode, RelayView, SettingsDetailKind, SettingsFocus,
    };
    use crate::app::App;
    use crossterm::event::KeyCode;

    pub(crate) fn settings_search_enter_opens_first_match_directly() {
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

    pub(crate) fn settings_search_enter_with_no_match_stays_in_list() {
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

    pub(crate) fn settings_search_allows_arrow_navigation_without_query() {
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

    pub(crate) fn settings_search_allows_arrow_navigation_with_filtered_results() {
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

    pub(crate) fn settings_numeric_shortcuts_and_detail_search_shortcut_keep_current_behavior() {
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

    pub(crate) fn settings_f1_closes_modal_from_detail_view() {
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

    pub(crate) fn settings_search_can_route_to_codex_relay_agent() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::List;
        app.settings_searching = true;
        app.settings_search = "codex relay".into();
        app.settings_selected = 0;

        let items = app.filtered_settings_items();
        assert_eq!(items.first().map(|item| item.0), Some("relay"));

        handle_settings_mode(&mut app, KeyCode::Enter);

        assert!(matches!(
            app.current_settings_detail_kind(),
            Some(SettingsDetailKind::Relay)
        ));
        assert!(matches!(app.relay_view, RelayView::ProviderList));
        assert_eq!(app.config.agents[app.relay_selected_agent].name, "codex");
    }

    pub(crate) fn settings_search_can_route_to_codex_settings_subpage() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::List;
        app.settings_searching = true;
        app.settings_search = "codex cli".into();
        app.settings_selected = 0;

        let items = app.filtered_settings_items();
        assert_eq!(items.first().map(|item| item.0), Some("codex_settings"));

        handle_settings_mode(&mut app, KeyCode::Enter);

        assert!(matches!(
            app.current_settings_detail_kind(),
            Some(SettingsDetailKind::CodexSettings)
        ));
        assert_eq!(app.codex_settings_view, CodexSettingsView::Cli);
    }
}
pub(crate) mod sound {
    use super::super::handle_settings_mode;
    use super::support::with_temp_home;
    use crate::app::state::{Mode, SettingsDetailKind, SettingsFocus};
    use crate::app::App;
    use crossterm::event::KeyCode;

    pub(crate) fn sound_settings_toggle_cycle_and_preview_work() {
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
}
mod support {
    pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
        crate::test_support::with_temp_home("pad-settings", name, |_| f())
    }
}
pub(crate) mod telegram {
    use super::super::handle_settings_mode;
    use super::support::with_temp_home;
    use crate::app::state::{Mode, SettingsDetailKind, SettingsFocus};
    use crate::app::App;
    use crossterm::event::KeyCode;

    pub(crate) fn telegram_settings_toggle_and_edit_fields_persist_without_leaving_detail() {
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

    pub(crate) fn telegram_settings_edit_escape_discards_buffer_and_r_keeps_detail_open() {
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
