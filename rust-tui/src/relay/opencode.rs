use super::common::{opencode_config_path, read_json_object_for_update, write_json_value};
use crate::theme::AgentConfig;
use serde_json::json;
use std::collections::BTreeSet;

mod managed;
mod model;
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

    write_json_value(&path, &root);
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
