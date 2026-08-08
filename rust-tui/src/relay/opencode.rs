use super::common::{
    log_file_error, opencode_config_path, read_json_object_for_update, write_json_value,
};
use crate::theme::AgentConfig;
use serde_json::json;
use std::collections::BTreeSet;

mod managed {
    use crate::relay::common::{
        log_file_error, opencode_managed_state_path, read_json_value, write_json_value,
    };
    use serde_json::json;
    use std::collections::BTreeSet;

    pub(super) fn read_opencode_managed_keys() -> BTreeSet<String> {
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

    pub(super) fn write_opencode_managed_keys(keys: &BTreeSet<String>) {
        let value = json!({
            "provider_keys": keys.iter().collect::<Vec<_>>()
        });
        let path = opencode_managed_state_path();
        if let Err(error) = write_json_value(&path, &value) {
            log_file_error("write", &path, &error);
        }
    }
}
mod model {
    use std::collections::BTreeSet;

    pub(super) fn sync_model_ref(
        root: &mut serde_json::Value,
        field: &str,
        selected_model: &str,
        valid_models: &BTreeSet<String>,
        previous_managed: &BTreeSet<String>,
    ) {
        if !selected_model.trim().is_empty() && valid_models.contains(selected_model) {
            root.as_object_mut().expect("opencode root object").insert(
                field.to_string(),
                serde_json::Value::String(selected_model.to_string()),
            );
        } else if model_ref_was_managed(root, field, previous_managed) {
            root.as_object_mut()
                .expect("opencode root object")
                .remove(field);
        }
    }

    fn model_ref_was_managed(
        root: &serde_json::Value,
        field: &str,
        previous_managed: &BTreeSet<String>,
    ) -> bool {
        root.get(field)
            .and_then(|value| value.as_str())
            .map(|value| {
                previous_managed
                    .iter()
                    .any(|key| value.starts_with(&format!("{key}/")))
            })
            .unwrap_or(false)
    }
}
mod provider {
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
}

pub(super) fn apply_opencode_agent_config(agent: &AgentConfig) {
    let path = opencode_config_path();
    let Some(mut root) = read_json_object_for_update(
        &path,
        json!({ "$schema": "https://opencode.ai/config.json" }),
    ) else {
        return;
    };
    ensure_schema(&mut root);

    let previous_managed = managed::read_opencode_managed_keys();
    let current_managed = provider::current_managed_provider_keys(agent);
    provider::sync_provider_map(&mut root, agent, &previous_managed, &current_managed);

    let valid_models: BTreeSet<String> = agent
        .opencode_model_options()
        .into_iter()
        .map(|(value, _)| value)
        .collect();
    model::sync_model_ref(
        &mut root,
        "model",
        &agent.default_model,
        &valid_models,
        &previous_managed,
    );
    model::sync_model_ref(
        &mut root,
        "small_model",
        &agent.small_model,
        &valid_models,
        &previous_managed,
    );

    if let Err(error) = write_json_value(&path, &root) {
        log_file_error("write", &path, &error);
        return;
    }
    managed::write_opencode_managed_keys(&current_managed);
}

fn ensure_schema(root: &mut serde_json::Value) {
    if root.get("$schema").is_some() {
        return;
    }

    root.as_object_mut()
        .expect("opencode config object")
        .insert(
            "$schema".to_string(),
            serde_json::Value::String("https://opencode.ai/config.json".to_string()),
        );
}
