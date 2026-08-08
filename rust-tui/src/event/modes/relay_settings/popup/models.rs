mod edit {
    use super::super::super::edit::persist_relay_config;
    use super::selection::{selected_provider_model, unique_model_id};
    use crate::app::App;

    pub(super) fn open_opencode_model_field_edit(app: &mut App) {
        if let Some(model) = selected_provider_model(app) {
            app.relay_popup_buffer = if app.relay_popup_field == 0 {
                model.id.clone()
            } else {
                model.name.clone()
            };
            app.relay_popup_editing = true;
        }
    }

    pub(super) fn commit_opencode_model_field_edit(app: &mut App) {
        let agent_idx = app.relay_selected_agent;
        let prov_idx = app.relay_selected_provider;
        let model_idx = app.relay_popup_selected;
        let field = app.relay_popup_field;
        let value = app.relay_popup_buffer.trim().to_string();

        let prepared_model_id = if field == 0 && !value.is_empty() {
            app.config
                .agents
                .get(agent_idx)
                .and_then(|agent| agent.providers.get(prov_idx))
                .map(|provider| unique_model_id(provider, &value, Some(model_idx)))
        } else {
            None
        };

        if let Some(agent) = app.config.agents.get_mut(agent_idx) {
            let provider_key = agent
                .providers
                .get(prov_idx)
                .map(|provider| provider.opencode_provider_key().to_string())
                .unwrap_or_default();
            let mut rename: Option<(String, String)> = None;

            if let Some(provider) = agent.providers.get_mut(prov_idx) {
                if field == 0 {
                    if let Some(new_id) = prepared_model_id {
                        if let Some(model) = provider.models.get_mut(model_idx) {
                            let old_id = model.id.clone();
                            model.id = new_id.clone();
                            rename = Some((old_id, new_id));
                        }
                    }
                } else if let Some(model) = provider.models.get_mut(model_idx) {
                    model.name = value;
                }
            }

            if let Some((old_id, new_id)) = rename {
                agent.rename_opencode_model_id(&provider_key, &old_id, &new_id);
            }
            agent.repair_opencode_model_refs();
        }

        persist_relay_config(app, agent_idx);
        app.relay_popup_editing = false;
        app.relay_popup_buffer.clear();
    }
}
mod modify {
    use super::super::super::edit::persist_relay_config;
    use super::edit::open_opencode_model_field_edit;
    use super::selection::unique_model_id;
    use crate::app::App;
    use crate::theme::OpenCodeModelConfig;

    pub(super) fn add_opencode_model(app: &mut App) {
        let agent_idx = app.relay_selected_agent;
        let prov_idx = app.relay_selected_provider;
        if let Some(agent) = app.config.agents.get_mut(agent_idx) {
            if let Some(provider) = agent.providers.get_mut(prov_idx) {
                let model_id = unique_model_id(provider, "model-1", None);
                provider.models.push(OpenCodeModelConfig {
                    id: model_id,
                    name: "Model".to_string(),
                });
                app.relay_popup_selected = provider.models.len().saturating_sub(1);
                app.relay_popup_field = 0;
                agent.repair_opencode_model_refs();
            }
        }
        persist_relay_config(app, agent_idx);
        open_opencode_model_field_edit(app);
    }

    pub(super) fn delete_opencode_model(app: &mut App) {
        let agent_idx = app.relay_selected_agent;
        let prov_idx = app.relay_selected_provider;
        let model_idx = app.relay_popup_selected;
        if let Some(agent) = app.config.agents.get_mut(agent_idx) {
            if let Some(provider) = agent.providers.get_mut(prov_idx) {
                if model_idx < provider.models.len() {
                    provider.models.remove(model_idx);
                    if app.relay_popup_selected > 0
                        && app.relay_popup_selected >= provider.models.len()
                    {
                        app.relay_popup_selected = provider.models.len().saturating_sub(1);
                    }
                    agent.repair_opencode_model_refs();
                }
            }
        }
        persist_relay_config(app, agent_idx);
    }
}
mod selection {
    use crate::app::App;
    use crate::theme::{OpenCodeModelConfig, ProviderConfig};

    pub(super) fn selected_provider_models_len(app: &App) -> Option<usize> {
        app.config
            .agents
            .get(app.relay_selected_agent)?
            .providers
            .get(app.relay_selected_provider)
            .map(|provider| provider.models.len())
    }

    pub(super) fn selected_provider_model(app: &App) -> Option<&OpenCodeModelConfig> {
        app.config
            .agents
            .get(app.relay_selected_agent)?
            .providers
            .get(app.relay_selected_provider)?
            .models
            .get(app.relay_popup_selected)
    }

    pub(super) fn unique_model_id(
        provider: &ProviderConfig,
        raw: &str,
        skip_idx: Option<usize>,
    ) -> String {
        let base = raw.trim();
        let base = if base.is_empty() { "model-1" } else { base };
        let mut candidate = base.to_string();
        let mut suffix = 2usize;
        loop {
            let conflict = provider.models.iter().enumerate().any(|(idx, model)| {
                if Some(idx) == skip_idx {
                    return false;
                }
                model.id == candidate
            });
            if !conflict {
                return candidate;
            }
            candidate = format!("{base}-{suffix}");
            suffix += 1;
        }
    }
}

use crate::app::App;
use crossterm::event::KeyCode;

pub(super) fn handle_relay_popup_edit(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => {
            app.relay_popup_editing = false;
            app.relay_popup_buffer.clear();
        }
        KeyCode::Enter => edit::commit_opencode_model_field_edit(app),
        KeyCode::Backspace => {
            app.relay_popup_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.relay_popup_buffer.push(c);
        }
        _ => {}
    }
    app.dirty = true;
    true
}

pub(super) fn handle_opencode_models_popup(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.clear_relay_popup_state();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(models_len) = selection::selected_provider_models_len(app) {
                let max = models_len.saturating_sub(1);
                if app.relay_popup_selected < max {
                    app.relay_popup_selected += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up if app.relay_popup_selected > 0 => {
            app.relay_popup_selected -= 1;
        }
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
            app.relay_popup_field = (app.relay_popup_field + 1) % 2;
        }
        KeyCode::Enter => {
            edit::open_opencode_model_field_edit(app);
        }
        KeyCode::Char('a') => {
            modify::add_opencode_model(app);
        }
        KeyCode::Char('d') => {
            modify::delete_opencode_model(app);
        }
        _ => {}
    }
    app.dirty = true;
    true
}
