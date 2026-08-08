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
mod provider;

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
