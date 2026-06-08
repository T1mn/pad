use super::super::handle_settings_mode;
use super::support::with_temp_home;
use crate::app::state::{Mode, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crossterm::event::KeyCode;

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
