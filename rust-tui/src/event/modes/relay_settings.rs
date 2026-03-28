use crate::app::state::Mode;
use crate::app::state::RelayView;
use crate::app::App;
use crate::relay;
use crossterm::event::KeyCode;

pub(crate) fn handle_relay_settings_mode(app: &mut App, key: KeyCode) {
    let relay_field_count = |app: &App| -> usize {
        match app
            .config
            .agents
            .get(app.relay_selected_agent)
            .map(|agent| agent.name.as_str())
        {
            Some("codex") => 3,
            _ => 3,
        }
    };

    if app.relay_editing {
        match key {
            KeyCode::Esc => {
                app.relay_editing = false;
                app.relay_edit_buffer.clear();
                app.dirty = true;
            }
            KeyCode::Enter => {
                let agent_idx = app.relay_selected_agent;
                let prov_idx = app.relay_selected_provider;
                let field = app.relay_edit_field;
                let value = app.relay_edit_buffer.clone();
                if let Some(agent) = app.config.agents.get_mut(agent_idx) {
                    if let Some(prov) = agent.providers.get_mut(prov_idx) {
                        match field {
                            0 => prov.label = value,
                            1 => prov.base_url = value,
                            2 => {
                                prov.api_key = value;
                                if agent.name == "codex" {
                                    prov.env_key.clear();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                app.config.save();
                app.relay_editing = false;
                app.relay_edit_buffer.clear();
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                app.relay_edit_buffer.push(c);
                app.dirty = true;
            }
            KeyCode::Backspace => {
                app.relay_edit_buffer.pop();
                app.dirty = true;
            }
            _ => {}
        }
        return;
    }

    match app.relay_view {
        RelayView::AgentList => match key {
            KeyCode::Esc => {
                app.mode = Mode::Settings;
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
            KeyCode::Enter => {
                app.relay_view = RelayView::ProviderList;
                app.relay_selected_provider = 0;
                app.dirty = true;
            }
            _ => {}
        },
        RelayView::ProviderList => match key {
            KeyCode::Esc => {
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
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                    if !agent.providers.is_empty() {
                        app.relay_view = RelayView::DetailPane;
                        app.relay_edit_field = 0;
                    }
                }
                app.dirty = true;
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
                        app.config.save();
                        relay::apply_relay_configs(&app.config.agents);
                    }
                }
                app.dirty = true;
            }
            KeyCode::Char('a') => {
                use crate::theme::ProviderConfig;
                if let Some(agent) = app.config.agents.get_mut(app.relay_selected_agent) {
                    agent.providers.push(ProviderConfig {
                        label: format!("provider-{}", agent.providers.len() + 1),
                        base_url: String::new(),
                        api_key: String::new(),
                        env_key: String::new(),
                        wire_api: "responses".to_string(),
                        test_status: None,
                        test_http_status: None,
                        test_latency_ms: None,
                        test_result: None,
                    });
                    app.relay_selected_provider = agent.providers.len() - 1;
                    app.config.save();
                }
                app.dirty = true;
            }
            KeyCode::Char('d') => {
                let agent_idx = app.relay_selected_agent;
                let prov_idx = app.relay_selected_provider;
                if let Some(agent) = app.config.agents.get_mut(agent_idx) {
                    if prov_idx < agent.providers.len() {
                        agent.providers.remove(prov_idx);
                        match agent.active_provider {
                            Some(i) if i == prov_idx => agent.active_provider = None,
                            Some(i) if i > prov_idx => agent.active_provider = Some(i - 1),
                            _ => {}
                        }
                        if app.relay_selected_provider > 0
                            && app.relay_selected_provider >= agent.providers.len()
                        {
                            app.relay_selected_provider = agent.providers.len().saturating_sub(1);
                        }
                        app.config.save();
                    }
                }
                app.dirty = true;
            }
            _ => {}
        },
        RelayView::DetailPane => match key {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                app.relay_view = RelayView::ProviderList;
                app.dirty = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.relay_edit_field = (app.relay_edit_field + 1) % relay_field_count(app);
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = relay_field_count(app);
                app.relay_edit_field = (app.relay_edit_field + count - 1) % count;
                app.dirty = true;
            }
            KeyCode::Enter => {
                if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                    if let Some(prov) = agent.providers.get(app.relay_selected_provider) {
                        app.relay_edit_buffer = match app.relay_edit_field {
                            0 => prov.label.clone(),
                            1 => prov.base_url.clone(),
                            2 => prov.api_key.clone(),
                            _ => String::new(),
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
            _ => {}
        },
    }
}
