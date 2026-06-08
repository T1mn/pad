use super::super::edit::{add_provider, delete_provider, persist_relay_config};
use super::super::{exit_relay, selected_agent_name, RelayHost};
use crate::app::state::{RelayPopupMode, RelayView};
use crate::app::App;
use crossterm::event::KeyCode;

pub(in crate::event::modes::relay_settings) fn handle_provider_list_key(
    app: &mut App,
    key: KeyCode,
    host: RelayHost,
) -> bool {
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
                    persist_relay_config(app, agent_idx);
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
            app.relay_popup_selected = super::super::popup::selected_model_picker_index(app, false);
            app.dirty = true;
        }
        KeyCode::Char('M') if selected_agent_name(app) == Some("opencode") => {
            app.relay_popup_mode = RelayPopupMode::OpenCodeSmallModel;
            app.relay_popup_selected = super::super::popup::selected_model_picker_index(app, true);
            app.dirty = true;
        }
        _ => {}
    }
    true
}
