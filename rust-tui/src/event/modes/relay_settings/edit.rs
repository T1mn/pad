mod field {
    use super::persist::persist_relay_config;
    use super::provider::uniquify_provider_key;
    use crate::app::App;
    use crossterm::event::KeyCode;

    pub(in crate::event::modes::relay_settings) fn handle_relay_field_edit(
        app: &mut App,
        key: KeyCode,
    ) -> bool {
        match key {
            KeyCode::Esc => {
                app.relay_editing = false;
                app.relay_edit_buffer.clear();
            }
            KeyCode::Enter => commit_relay_field_edit(app),
            KeyCode::Backspace => {
                app.relay_edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.relay_edit_buffer.push(c);
            }
            _ => {}
        }
        app.dirty = true;
        true
    }

    fn commit_relay_field_edit(app: &mut App) {
        let agent_idx = app.relay_selected_agent;
        let prov_idx = app.relay_selected_provider;
        let field = app.relay_edit_field;
        let value = app.relay_edit_buffer.trim().to_string();

        let prepared_provider_key = if field == 1 {
            app.config
                .agents
                .get(agent_idx)
                .map(|agent| uniquify_provider_key(agent, &value, Some(prov_idx)))
        } else {
            None
        };

        if let Some(agent) = app.config.agents.get_mut(agent_idx) {
            let mut provider_key_rename: Option<(String, String)> = None;
            let agent_name = agent.name.clone();
            if let Some(provider) = agent.providers.get_mut(prov_idx) {
                match agent_name.as_str() {
                    "opencode" => match field {
                        0 => provider.label = value,
                        1 => {
                            let old_key = provider.provider_key.clone();
                            let new_key = prepared_provider_key.unwrap_or_else(|| old_key.clone());
                            provider.provider_key = new_key.clone();
                            provider_key_rename = Some((old_key, new_key));
                        }
                        2 => {
                            provider.npm_package = if value.is_empty() {
                                "@ai-sdk/openai-compatible".to_string()
                            } else {
                                value
                            };
                        }
                        3 => provider.base_url = value,
                        4 => provider.api_key = value,
                        _ => {}
                    },
                    "codex" => match field {
                        0 => provider.label = value,
                        1 => provider.base_url = value,
                        2 => {
                            provider.api_key = value;
                            provider.env_key.clear();
                        }
                        _ => {}
                    },
                    _ => match field {
                        0 => provider.label = value,
                        1 => provider.base_url = value,
                        2 => provider.api_key = value,
                        3 if agent_name == "claude" => {
                            provider.disable_thinking = parse_bool_field(&value);
                        }
                        _ => {}
                    },
                }
            }

            if let Some((old_key, new_key)) = provider_key_rename {
                agent.rename_opencode_provider_key(&old_key, &new_key);
            }
            if agent_name == "opencode" {
                agent.repair_opencode_model_refs();
            }
        }

        persist_relay_config(app, agent_idx);
        app.relay_editing = false;
        app.relay_edit_buffer.clear();
    }

    fn parse_bool_field(value: &str) -> bool {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    }
}
mod persist {
    use crate::app::App;
    use crate::relay;

    pub(in crate::event::modes::relay_settings) fn persist_relay_config(
        app: &mut App,
        agent_idx: usize,
    ) {
        if let Some(agent) = app.config.agents.get_mut(agent_idx) {
            if agent.name == "opencode" {
                agent.repair_opencode_model_refs();
            }
        }
        app.save_config();
        relay::apply_runtime_configs(
            &app.config.agents,
            &app.config.agent_permissions,
            &app.config.codex,
        );
    }
}
mod provider {
    use super::persist::persist_relay_config;
    use crate::app::App;
    use crate::theme::{normalize_provider_key, OpenCodeModelConfig, ProviderConfig};

    pub(in crate::event::modes::relay_settings) fn add_provider(app: &mut App) {
        let agent_idx = app.relay_selected_agent;
        if let Some(agent) = app.config.agents.get_mut(agent_idx) {
            let label = format!("provider-{}", agent.providers.len() + 1);
            let provider_key = uniquify_provider_key(agent, &label, None);
            let mut provider = ProviderConfig {
                label,
                base_url: String::new(),
                api_key: String::new(),
                env_key: String::new(),
                wire_api: "responses".to_string(),
                provider_key,
                npm_package: "@ai-sdk/openai-compatible".to_string(),
                disable_thinking: false,
                models: Vec::new(),
                test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            };
            if agent.name == "opencode" {
                provider.models.push(OpenCodeModelConfig {
                    id: "model-1".to_string(),
                    name: "Model 1".to_string(),
                });
            }
            agent.providers.push(provider);
            app.relay_selected_provider = agent.providers.len().saturating_sub(1);
            if agent.name == "opencode" {
                agent.repair_opencode_model_refs();
            }
        }
        persist_relay_config(app, agent_idx);
    }

    pub(in crate::event::modes::relay_settings) fn delete_provider(app: &mut App) {
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
                if agent.name == "opencode" {
                    agent.repair_opencode_model_refs();
                }
                if app.relay_selected_provider > 0
                    && app.relay_selected_provider >= agent.providers.len()
                {
                    app.relay_selected_provider = agent.providers.len().saturating_sub(1);
                }
            }
        }
        persist_relay_config(app, agent_idx);
    }

    pub(in crate::event::modes::relay_settings) fn uniquify_provider_key(
        agent: &crate::theme::AgentConfig,
        raw: &str,
        skip_idx: Option<usize>,
    ) -> String {
        let base = normalize_provider_key(raw);
        let mut candidate = base.clone();
        let mut suffix = 2usize;
        loop {
            let conflict = agent.providers.iter().enumerate().any(|(idx, provider)| {
                if Some(idx) == skip_idx {
                    return false;
                }
                provider.opencode_provider_key() == candidate
            });
            if !conflict {
                return candidate;
            }
            candidate = format!("{base}-{suffix}");
            suffix += 1;
        }
    }
}

pub(super) use field::handle_relay_field_edit;
pub(super) use persist::persist_relay_config;
pub(super) use provider::{add_provider, delete_provider};
