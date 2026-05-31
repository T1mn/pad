use super::*;

#[test]
fn root_groups_options_and_back_navigation() {
    with_temp_home("root", || {
        let mut app = codex_settings_app();
        app.codex_settings_view = CodexSettingsView::Categories;
        app.codex_settings_selected = 0;
        app.codex_settings_category_selected = 0;

        handle_settings_mode(&mut app, KeyCode::Up);
        assert_eq!(app.codex_settings_selected, 0);
        assert_eq!(app.codex_settings_category_selected, 0);

        for _ in 0..CodexSettingsView::CATEGORY_COUNT + 2 {
            handle_settings_mode(&mut app, KeyCode::Down);
        }
        assert_eq!(
            app.codex_settings_selected,
            CodexSettingsView::CATEGORY_COUNT - 1
        );
        assert_eq!(
            app.codex_settings_category_selected,
            CodexSettingsView::CATEGORY_COUNT - 1
        );

        app.codex_settings_selected = 0;
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert_eq!(app.codex_settings_view, CodexSettingsView::Runtime);
        assert_eq!(app.codex_settings_selected, 0);
        assert_eq!(app.codex_settings_category_selected, 0);

        handle_settings_mode(&mut app, KeyCode::Esc);
        assert_eq!(app.codex_settings_view, CodexSettingsView::Categories);
        assert_eq!(app.codex_settings_selected, 0);
        assert!(matches!(app.settings_focus, SettingsFocus::Detail));

        handle_settings_mode(&mut app, KeyCode::Esc);
        assert!(matches!(app.settings_focus, SettingsFocus::List));
        assert!(app.active_settings_detail.is_none());
    });
}

#[test]
fn cli_category_keeps_check_update_actions_separate_from_config_toggles() {
    with_temp_home("cli", || {
        let mut app = codex_settings_app();
        open_codex_category(&mut app, 4);
        assert_eq!(app.codex_settings_view, CodexSettingsView::Cli);
        assert_eq!(app.codex_settings_selected, 0);

        handle_settings_mode(&mut app, KeyCode::Down);
        assert_eq!(app.codex_settings_selected, 0);
        handle_settings_mode(&mut app, KeyCode::Up);
        assert_eq!(app.codex_settings_selected, 0);

        handle_settings_mode(&mut app, KeyCode::Char('h'));
        assert_eq!(app.codex_settings_view, CodexSettingsView::Categories);
        assert_eq!(app.codex_settings_selected, 4);
    });
}
