use super::common::{
    opencode_config_path, opencode_managed_state_path, read_json_value, write_json_value,
};
use crate::theme::AgentConfig;
use serde_json::json;
use std::collections::BTreeSet;

fn read_opencode_managed_keys() -> BTreeSet<String> {
    let state_path = opencode_managed_state_path();
    let value = read_json_value(&state_path, json!({ "provider_keys": [] }));
    value
        .get("provider_keys")
        .and_then(|items| items.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect()
}

fn write_opencode_managed_keys(keys: &BTreeSet<String>) {
    let value = json!({
        "provider_keys": keys.iter().collect::<Vec<_>>()
    });
    write_json_value(&opencode_managed_state_path(), &value);
}

pub(super) fn apply_opencode_agent_config(agent: &AgentConfig) {
    let path = opencode_config_path();
    let mut root = read_json_value(
        &path,
        json!({ "$schema": "https://opencode.ai/config.json" }),
    );

    if root.get("$schema").is_none() {
        root.as_object_mut()
            .expect("opencode config object")
            .insert(
                "$schema".to_string(),
                serde_json::Value::String("https://opencode.ai/config.json".to_string()),
            );
    }

    let previous_managed = read_opencode_managed_keys();
    let current_managed: BTreeSet<String> = agent
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
        .collect();

    let provider_entry = root
        .as_object_mut()
        .expect("opencode root object")
        .entry("provider".to_string())
        .or_insert_with(|| json!({}));
    if !provider_entry.is_object() {
        *provider_entry = json!({});
    }
    let provider_map = provider_entry
        .as_object_mut()
        .expect("opencode provider object");

    for removed_key in previous_managed.difference(&current_managed) {
        provider_map.remove(removed_key);
    }

    for provider in &agent.providers {
        let provider_key = provider.opencode_provider_key().trim();
        if provider_key.is_empty() {
            continue;
        }

        let models = provider
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
            .collect::<serde_json::Map<String, serde_json::Value>>();

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

        let config = json!({
            "npm": provider.opencode_npm_package(),
            "name": provider.label,
            "options": options,
            "models": models,
        });
        provider_map.insert(provider_key.to_string(), config);
    }

    let valid_models: BTreeSet<String> = agent
        .opencode_model_options()
        .into_iter()
        .map(|(value, _)| value)
        .collect();

    if !agent.default_model.trim().is_empty() && valid_models.contains(&agent.default_model) {
        root.as_object_mut().expect("opencode root object").insert(
            "model".to_string(),
            serde_json::Value::String(agent.default_model.clone()),
        );
    } else if root
        .get("model")
        .and_then(|value| value.as_str())
        .map(|value| {
            previous_managed
                .iter()
                .any(|key| value.starts_with(&format!("{key}/")))
        })
        .unwrap_or(false)
    {
        root.as_object_mut()
            .expect("opencode root object")
            .remove("model");
    }

    if !agent.small_model.trim().is_empty() && valid_models.contains(&agent.small_model) {
        root.as_object_mut().expect("opencode root object").insert(
            "small_model".to_string(),
            serde_json::Value::String(agent.small_model.clone()),
        );
    } else if root
        .get("small_model")
        .and_then(|value| value.as_str())
        .map(|value| {
            previous_managed
                .iter()
                .any(|key| value.starts_with(&format!("{key}/")))
        })
        .unwrap_or(false)
    {
        root.as_object_mut()
            .expect("opencode root object")
            .remove("small_model");
    }

    write_json_value(&path, &root);
    write_opencode_managed_keys(&current_managed);
}
