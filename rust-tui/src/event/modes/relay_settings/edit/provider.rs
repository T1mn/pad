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
