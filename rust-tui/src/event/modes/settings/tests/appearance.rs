use super::super::handle_settings_mode;
use super::support::with_temp_home;
use crate::app::state::{Mode, SettingsFocus};
use crate::app::App;
use crossterm::event::KeyCode;

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
