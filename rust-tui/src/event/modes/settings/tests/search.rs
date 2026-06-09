use super::super::handle_settings_mode;
use crate::app::state::{CodexSettingsView, Mode, RelayView, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crossterm::event::KeyCode;

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
fn settings_search_can_route_to_codex_relay_agent() {
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

#[test]
fn settings_search_can_route_to_codex_settings_subpage() {
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
