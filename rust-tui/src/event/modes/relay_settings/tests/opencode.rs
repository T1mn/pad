use super::super::{handle_relay_key, RelayHost};
use super::support::{agent_index, sample_provider, with_temp_home};
use crate::app::state::{RelayPopupMode, RelayView};
use crate::app::App;
use crate::theme::{OpenCodeModelConfig, ProviderConfig};
use crossterm::event::KeyCode;

#[test]
fn opencode_small_model_picker_can_clear_selection() {
    with_temp_home("opencode-small-model", || {
        let mut app = App::new();
        let opencode_idx = agent_index(&app, "opencode");
        let agent = &mut app.config.agents[opencode_idx];
        agent.providers.push(ProviderConfig {
            models: vec![OpenCodeModelConfig {
                id: "gpt-5".into(),
                name: "GPT-5".into(),
            }],
            ..sample_provider("relay")
        });
        agent.small_model = "relay/gpt-5".into();

        app.relay_selected_agent = opencode_idx;
        app.relay_selected_provider = 0;
        app.relay_view = RelayView::ProviderList;

        handle_relay_key(&mut app, KeyCode::Char('M'), RelayHost::Standalone);
        assert!(matches!(
            app.relay_popup_mode,
            RelayPopupMode::OpenCodeSmallModel
        ));

        app.relay_popup_selected = 0;
        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);

        assert_eq!(app.config.agents[opencode_idx].small_model, "");
        assert!(matches!(app.relay_popup_mode, RelayPopupMode::None));
    });
}

#[test]
fn opencode_model_popup_supports_add_edit_and_delete_flow() {
    with_temp_home("opencode-model-popup", || {
        let mut app = App::new();
        let opencode_idx = agent_index(&app, "opencode");
        let agent = &mut app.config.agents[opencode_idx];
        agent.providers.push(ProviderConfig {
            models: vec![OpenCodeModelConfig {
                id: "model-1".into(),
                name: "Model 1".into(),
            }],
            ..sample_provider("relay")
        });

        app.relay_selected_agent = opencode_idx;
        app.relay_selected_provider = 0;
        app.relay_view = RelayView::DetailPane;
        app.relay_edit_field = 5;

        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);
        assert!(matches!(
            app.relay_popup_mode,
            RelayPopupMode::OpenCodeModels
        ));

        handle_relay_key(&mut app, KeyCode::Char('a'), RelayHost::Standalone);
        assert!(app.relay_popup_editing);
        assert_eq!(app.relay_popup_selected, 1);
        assert_eq!(app.config.agents[opencode_idx].providers[0].models.len(), 2);

        app.relay_popup_buffer = "custom-model".into();
        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);
        assert!(!app.relay_popup_editing);
        assert_eq!(
            app.config.agents[opencode_idx].providers[0].models[1].id,
            "custom-model"
        );

        app.relay_popup_field = 1;
        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);
        assert!(app.relay_popup_editing);
        app.relay_popup_buffer = "Custom Model".into();
        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);
        assert_eq!(
            app.config.agents[opencode_idx].providers[0].models[1].name,
            "Custom Model"
        );

        handle_relay_key(&mut app, KeyCode::Char('d'), RelayHost::Standalone);
        assert_eq!(app.config.agents[opencode_idx].providers[0].models.len(), 1);
        assert_eq!(app.relay_popup_selected, 0);
    });
}

#[test]
fn opencode_model_id_edit_is_uniquified_and_updates_model_refs() {
    with_temp_home("opencode-model-id-edit", || {
        let mut app = App::new();
        let opencode_idx = agent_index(&app, "opencode");
        let agent = &mut app.config.agents[opencode_idx];
        agent.providers.push(ProviderConfig {
            models: vec![
                OpenCodeModelConfig {
                    id: "gpt-5".into(),
                    name: "GPT-5".into(),
                },
                OpenCodeModelConfig {
                    id: "gpt-5-mini".into(),
                    name: "GPT-5 Mini".into(),
                },
            ],
            ..sample_provider("relay")
        });
        agent.default_model = "relay/gpt-5".into();
        agent.small_model = "relay/gpt-5".into();

        app.relay_selected_agent = opencode_idx;
        app.relay_selected_provider = 0;
        app.relay_view = RelayView::DetailPane;
        app.relay_edit_field = 5;

        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);
        app.relay_popup_selected = 0;
        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);
        assert!(app.relay_popup_editing);

        app.relay_popup_buffer = "gpt-5-mini".into();
        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);

        let agent = &app.config.agents[opencode_idx];
        assert_eq!(agent.providers[0].models[0].id, "gpt-5-mini-2");
        assert_eq!(agent.default_model, "relay/gpt-5-mini-2");
        assert_eq!(agent.small_model, "relay/gpt-5-mini-2");
        assert!(!app.relay_popup_editing);
    });
}

#[test]
fn opencode_provider_key_edit_is_uniquified_and_updates_model_refs() {
    with_temp_home("opencode-provider-key-edit", || {
        let mut app = App::new();
        let opencode_idx = agent_index(&app, "opencode");
        let agent = &mut app.config.agents[opencode_idx];
        agent.providers.push(ProviderConfig {
            models: vec![OpenCodeModelConfig {
                id: "gpt-5".into(),
                name: "GPT-5".into(),
            }],
            ..sample_provider("relay-a")
        });
        agent.providers.push(ProviderConfig {
            models: vec![OpenCodeModelConfig {
                id: "gpt-5-mini".into(),
                name: "GPT-5 Mini".into(),
            }],
            ..sample_provider("relay-b")
        });
        agent.default_model = "relay-a/gpt-5".into();
        agent.small_model = "relay-a/gpt-5".into();

        app.relay_selected_agent = opencode_idx;
        app.relay_selected_provider = 0;
        app.relay_view = RelayView::DetailPane;
        app.relay_edit_field = 1;

        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);
        assert!(app.relay_editing);

        app.relay_edit_buffer = "relay-b".into();
        handle_relay_key(&mut app, KeyCode::Enter, RelayHost::Standalone);

        let agent = &app.config.agents[opencode_idx];
        assert_eq!(agent.providers[0].provider_key, "relay-b-2");
        assert_eq!(agent.default_model, "relay-b-2/gpt-5");
        assert_eq!(agent.small_model, "relay-b-2/gpt-5");
        assert!(!app.relay_editing);
    });
}
