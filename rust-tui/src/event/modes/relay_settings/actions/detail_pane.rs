use super::super::transfer::{export_selected_codex_provider, import_selected_codex_provider};
use super::super::{relay_field_count, selected_agent_name, RelayHost};
use crate::app::state::{RelayPopupMode, RelayView};
use crate::app::App;
use crossterm::event::KeyCode;

pub(in crate::event::modes::relay_settings) fn handle_detail_pane_key(
    app: &mut App,
    key: KeyCode,
    _host: RelayHost,
) -> bool {
    match key {
        KeyCode::Esc => {
            app.relay_view = RelayView::ProviderList;
            app.dirty = true;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.relay_view = RelayView::ProviderList;
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let count = relay_field_count(app);
            app.relay_edit_field = (app.relay_edit_field + 1) % count;
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let count = relay_field_count(app);
            app.relay_edit_field = (app.relay_edit_field + count - 1) % count;
            app.dirty = true;
        }
        KeyCode::Enter => {
            if selected_agent_name(app) == Some("opencode") && app.relay_edit_field == 5 {
                app.relay_popup_mode = RelayPopupMode::OpenCodeModels;
                app.relay_popup_selected = 0;
                app.relay_popup_field = 0;
                app.relay_popup_editing = false;
                app.relay_popup_buffer.clear();
            } else if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                if let Some(provider) = agent.providers.get(app.relay_selected_provider) {
                    app.relay_edit_buffer = match agent.name.as_str() {
                        "opencode" => match app.relay_edit_field {
                            0 => provider.label.clone(),
                            1 => provider.provider_key.clone(),
                            2 => provider.npm_package.clone(),
                            3 => provider.base_url.clone(),
                            4 => provider.api_key.clone(),
                            _ => String::new(),
                        },
                        _ => match app.relay_edit_field {
                            0 => provider.label.clone(),
                            1 => provider.base_url.clone(),
                            2 => provider.api_key.clone(),
                            3 if agent.name == "claude" => provider.disable_thinking.to_string(),
                            _ => String::new(),
                        },
                    };
                    app.relay_editing = true;
                }
            }
            app.dirty = true;
        }
        KeyCode::Char(' ') => {
            app.trigger_provider_test(app.relay_selected_agent, app.relay_selected_provider);
            app.dirty = true;
        }
        KeyCode::Char('y') | KeyCode::Char('Y') if selected_agent_name(app) == Some("codex") => {
            export_selected_codex_provider(app);
            app.dirty = true;
        }
        KeyCode::Char('i') | KeyCode::Char('I') if selected_agent_name(app) == Some("codex") => {
            import_selected_codex_provider(app);
            app.dirty = true;
        }
        _ => {}
    }
    true
}
