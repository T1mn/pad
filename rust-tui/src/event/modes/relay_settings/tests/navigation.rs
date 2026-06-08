use super::super::{handle_relay_key, RelayHost};
use crate::app::state::{Mode, RelayView, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crossterm::event::KeyCode;

#[test]
fn relay_escape_from_settings_host_returns_to_settings_list() {
    let mut app = App::new();
    app.mode = Mode::Settings;
    app.settings_open = true;
    app.settings_focus = SettingsFocus::Detail;
    app.active_settings_detail = Some(SettingsDetailKind::Relay);
    app.relay_view = RelayView::ProviderList;

    handle_relay_key(&mut app, KeyCode::Esc, RelayHost::Settings);

    assert!(matches!(app.mode, Mode::Settings));
    assert!(matches!(app.settings_focus, SettingsFocus::List));
    assert!(app.active_settings_detail.is_none());
}

#[test]
fn relay_escape_from_standalone_provider_list_returns_to_agent_list() {
    let mut app = App::new();
    app.mode = Mode::RelaySettings;
    app.relay_view = RelayView::ProviderList;

    handle_relay_key(&mut app, KeyCode::Esc, RelayHost::Standalone);

    assert!(matches!(app.mode, Mode::RelaySettings));
    assert!(matches!(app.relay_view, RelayView::AgentList));
}
