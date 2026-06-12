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
