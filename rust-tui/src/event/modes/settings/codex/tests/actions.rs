use super::*;

#[test]
fn runtime_category_preserves_original_switches_and_web_search_cycle() {
    with_temp_home("runtime", || {
        let mut app = codex_settings_app();
        app.config.agent_permissions.codex_auto_full_access = false;
        app.config.codex.fast_mode = false;
        app.config.codex.goals = false;
        app.config.codex.multi_agent = false;
        app.config.codex.web_search = "default".into();

        open_codex_category(&mut app, 0);
        assert_eq!(app.codex_settings_view, CodexSettingsView::Runtime);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.agent_permissions.codex_auto_full_access);

        handle_settings_mode(&mut app, KeyCode::Down);
        assert_eq!(app.codex_settings_selected, 1);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.fast_mode);

        handle_settings_mode(&mut app, KeyCode::Down);
        assert_eq!(app.codex_settings_selected, 2);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.goals);

        handle_settings_mode(&mut app, KeyCode::Down);
        assert_eq!(app.codex_settings_selected, 3);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.multi_agent);

        handle_settings_mode(&mut app, KeyCode::Down);
        assert_eq!(app.codex_settings_selected, 4);
        for expected in ["cached", "live", "disabled", "default"] {
            handle_settings_mode(&mut app, KeyCode::Enter);
            assert_eq!(app.config.codex.web_search, expected);
        }
        for _ in 0..3 {
            handle_settings_mode(&mut app, KeyCode::Down);
        }
        assert_eq!(app.codex_settings_selected, 4);
    });
}

#[test]
fn status_prompt_and_preview_categories_preserve_original_toggles() {
    with_temp_home("subcategories", || {
        let mut app = codex_settings_app();
        app.config.codex.status_line_model_with_reasoning = false;
        app.config.codex.status_line_fast_mode = false;
        app.config.codex.status_line_context_remaining = false;
        app.config.codex.status_line_current_dir = false;
        app.config.codex.jailbreak_prompt_file = false;
        app.config.codex.index_prompt_file = false;
        app.config.codex.title_summary = false;
        app.config.codex.show_qa_preview = false;

        open_codex_category(&mut app, 1);
        assert_eq!(app.codex_settings_view, CodexSettingsView::StatusLine);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.status_line_model_with_reasoning);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(!app.config.codex.status_line_model_with_reasoning);
        handle_settings_mode(&mut app, KeyCode::Down);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.status_line_fast_mode);
        handle_settings_mode(&mut app, KeyCode::Down);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.status_line_context_remaining);
        handle_settings_mode(&mut app, KeyCode::Down);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.status_line_current_dir);
        handle_settings_mode(&mut app, KeyCode::Char('h'));
        assert_eq!(app.codex_settings_selected, 1);

        open_codex_category(&mut app, 2);
        assert_eq!(app.codex_settings_view, CodexSettingsView::Prompts);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.jailbreak_prompt_file);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(!app.config.codex.jailbreak_prompt_file);
        handle_settings_mode(&mut app, KeyCode::Down);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.index_prompt_file);
        handle_settings_mode(&mut app, KeyCode::Char('h'));
        assert_eq!(app.codex_settings_selected, 2);

        open_codex_category(&mut app, 3);
        assert_eq!(app.codex_settings_view, CodexSettingsView::Preview);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.title_summary);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(!app.config.codex.title_summary);
        handle_settings_mode(&mut app, KeyCode::Down);
        handle_settings_mode(&mut app, KeyCode::Enter);
        assert!(app.config.codex.show_qa_preview);
    });
}
