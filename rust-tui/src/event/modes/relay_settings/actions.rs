use super::edit::{add_provider, delete_provider};
use super::transfer::{export_selected_codex_provider, import_selected_codex_provider};
use super::{exit_relay, relay_field_count, selected_agent_name, RelayHost};
use crate::app::state::{RelayPopupMode, RelayView};
use crate::app::App;
use crossterm::event::KeyCode;

pub(super) fn handle_agent_list_key(app: &mut App, key: KeyCode, host: RelayHost) -> bool {
    match key {
        KeyCode::Esc => {
            exit_relay(app, host);
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.config.agents.len().saturating_sub(1);
            if app.relay_selected_agent < max {
                app.relay_selected_agent += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.relay_selected_agent > 0 {
                app.relay_selected_agent -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            app.relay_view = RelayView::ProviderList;
            let active = app
                .config
                .agents
                .get(app.relay_selected_agent)
                .and_then(|agent| agent.active_provider)
                .unwrap_or(0);
            app.relay_selected_provider = active;
            app.dirty = true;
        }
        _ => {}
    }
    true
}

pub(super) fn handle_provider_list_key(app: &mut App, key: KeyCode, host: RelayHost) -> bool {
    match key {
        KeyCode::Esc => {
            if host == RelayHost::Settings {
                exit_relay(app, host);
            } else {
                app.relay_view = RelayView::AgentList;
            }
            app.dirty = true;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.relay_view = RelayView::AgentList;
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                let max = agent.providers.len().saturating_sub(1);
                if app.relay_selected_provider < max {
                    app.relay_selected_provider += 1;
                }
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.relay_selected_provider > 0 {
                app.relay_selected_provider -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
            if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                if !agent.providers.is_empty() {
                    app.relay_view = RelayView::DetailPane;
                    app.relay_edit_field = 0;
                    app.dirty = true;
                }
            }
        }
        KeyCode::Char(' ') => {
            let agent_idx = app.relay_selected_agent;
            let prov_idx = app.relay_selected_provider;
            if let Some(agent) = app.config.agents.get_mut(agent_idx) {
                if prov_idx < agent.providers.len() {
                    if agent.active_provider == Some(prov_idx) {
                        agent.active_provider = None;
                    } else {
                        agent.active_provider = Some(prov_idx);
                    }
                    super::edit::persist_relay_config(app, agent_idx);
                }
            }
            app.dirty = true;
        }
        KeyCode::Char('a') => {
            add_provider(app);
            app.dirty = true;
        }
        KeyCode::Char('d') => {
            delete_provider(app);
            app.dirty = true;
        }
        KeyCode::Char('m') if selected_agent_name(app) == Some("opencode") => {
            app.relay_popup_mode = RelayPopupMode::OpenCodeDefaultModel;
            app.relay_popup_selected = super::popup::selected_model_picker_index(app, false);
            app.dirty = true;
        }
        KeyCode::Char('M') if selected_agent_name(app) == Some("opencode") => {
            app.relay_popup_mode = RelayPopupMode::OpenCodeSmallModel;
            app.relay_popup_selected = super::popup::selected_model_picker_index(app, true);
            app.dirty = true;
        }
        _ => {}
    }
    true
}

pub(super) fn handle_detail_pane_key(app: &mut App, key: KeyCode, host: RelayHost) -> bool {
    match key {
        KeyCode::Esc => {
            if host == RelayHost::Settings {
                exit_relay(app, host);
            } else {
                app.relay_view = RelayView::ProviderList;
            }
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
