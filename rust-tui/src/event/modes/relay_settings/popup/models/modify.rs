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
                if app.relay_popup_selected > 0 && app.relay_popup_selected >= provider.models.len()
                {
                    app.relay_popup_selected = provider.models.len().saturating_sub(1);
                }
                agent.repair_opencode_model_refs();
            }
        }
    }
    persist_relay_config(app, agent_idx);
}
