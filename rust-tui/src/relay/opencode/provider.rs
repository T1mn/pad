use crate::theme::{AgentConfig, ProviderConfig};
use serde_json::json;
use std::collections::BTreeSet;

pub(super) fn current_managed_provider_keys(agent: &AgentConfig) -> BTreeSet<String> {
    agent
        .providers
        .iter()
        .filter_map(|provider| {
            let key = provider.opencode_provider_key().trim();
            if key.is_empty() {
                None
            } else {
                Some(key.to_string())
            }
        })
        .collect()
}

pub(super) fn sync_provider_map(
    root: &mut serde_json::Value,
    agent: &AgentConfig,
    previous_managed: &BTreeSet<String>,
    current_managed: &BTreeSet<String>,
) {
    let provider_map = provider_map_mut(root);

    for removed_key in previous_managed.difference(current_managed) {
        provider_map.remove(removed_key);
    }

    for provider in &agent.providers {
        let provider_key = provider.opencode_provider_key().trim();
        if provider_key.is_empty() {
            continue;
        }
        provider_map.insert(provider_key.to_string(), provider_config(provider));
    }
}

fn provider_map_mut(
    root: &mut serde_json::Value,
) -> &mut serde_json::Map<String, serde_json::Value> {
    let provider_entry = root
        .as_object_mut()
        .expect("opencode root object")
        .entry("provider".to_string())
        .or_insert_with(|| json!({}));
    if !provider_entry.is_object() {
        *provider_entry = json!({});
    }
    provider_entry
        .as_object_mut()
        .expect("opencode provider object")
}

fn provider_config(provider: &ProviderConfig) -> serde_json::Value {
    json!({
        "npm": provider.opencode_npm_package(),
        "name": provider.label,
        "options": provider_options(provider),
        "models": provider_models(provider),
    })
}

fn provider_models(provider: &ProviderConfig) -> serde_json::Map<String, serde_json::Value> {
    provider
        .models
        .iter()
        .filter(|model| !model.id.trim().is_empty())
        .map(|model| {
            let display_name = if model.name.trim().is_empty() {
                model.id.trim()
            } else {
                model.name.trim()
            };
            (
                model.id.trim().to_string(),
                json!({
                    "name": display_name,
                }),
            )
        })
        .collect()
}

fn provider_options(provider: &ProviderConfig) -> serde_json::Map<String, serde_json::Value> {
    let mut options = serde_json::Map::new();
    if !provider.base_url.trim().is_empty() {
        options.insert(
            "baseURL".to_string(),
            serde_json::Value::String(provider.base_url.trim().to_string()),
        );
    }
    if !provider.api_key.trim().is_empty() {
        options.insert(
            "apiKey".to_string(),
            serde_json::Value::String(provider.api_key.clone()),
        );
    }
    options
}
