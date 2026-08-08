mod navigation {
    use super::super::{handle_relay_key, RelayHost};
    use crate::app::state::{Mode, RelayView, SettingsDetailKind, SettingsFocus};
    use crate::app::App;
    use crossterm::event::KeyCode;

    #[test]
    fn relay_escape_from_settings_host_steps_back_by_level() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::Detail;
        app.active_settings_detail = Some(SettingsDetailKind::Relay);
        app.relay_view = RelayView::DetailPane;

        handle_relay_key(&mut app, KeyCode::Esc, RelayHost::Settings);
        assert!(matches!(app.relay_view, RelayView::ProviderList));
        assert!(matches!(app.settings_focus, SettingsFocus::Detail));

        handle_relay_key(&mut app, KeyCode::Esc, RelayHost::Settings);
        assert!(matches!(app.relay_view, RelayView::AgentList));
        assert!(matches!(app.settings_focus, SettingsFocus::Detail));

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
}
mod opencode;
mod provider;
mod support {
    use crate::app::App;
    use crate::theme::ProviderConfig;

    pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
        crate::test_support::with_temp_home("pad-relay-settings", name, |_| f())
    }

    pub(super) fn sample_provider(label: &str) -> ProviderConfig {
        ProviderConfig {
            label: label.to_string(),
            base_url: "https://example.test".to_string(),
            api_key: "secret".to_string(),
            env_key: String::new(),
            wire_api: "responses".to_string(),
            provider_key: label.to_string(),
            npm_package: "@ai-sdk/openai-compatible".to_string(),
            disable_thinking: false,
            models: Vec::new(),
            test_status: None,
            test_http_status: None,
            test_latency_ms: None,
            test_result: None,
        }
    }

    pub(super) fn agent_index(app: &App, name: &str) -> usize {
        app.config
            .agents
            .iter()
            .position(|agent| agent.name == name)
            .expect("agent")
    }
}
