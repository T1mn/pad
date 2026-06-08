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
