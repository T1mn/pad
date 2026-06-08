use super::super::super::common::{codex_permission_state_path, read_json_value, write_json_value};
use super::super::toml_helpers::{toml_bool_at_path, toml_string_array_at_path};
use serde_json::json;

pub(super) fn read_codex_permission_state() -> serde_json::Value {
    read_json_value(
        &codex_permission_state_path(),
        codex_permission_state_defaults(),
    )
}

fn codex_permission_state_defaults() -> serde_json::Value {
    json!({
        "approval_policy": null,
        "sandbox_mode": null,
        "service_tier": null,
        "features_fast_mode": null,
        "features_goals": null,
        "features_multi_agent": null,
        "web_search": null,
        "tui_status_line": null,
        "model_instructions_file": null
    })
}

pub(super) fn capture_codex_permission_state_once(root: &toml::map::Map<String, toml::Value>) {
    let path = codex_permission_state_path();
    if path.exists() {
        return;
    }

    let value = json!({
        "approval_policy": root.get("approval_policy").and_then(|value| value.as_str()),
        "sandbox_mode": root.get("sandbox_mode").and_then(|value| value.as_str()),
        "service_tier": root.get("service_tier").and_then(|value| value.as_str()),
        "features_fast_mode": toml_bool_at_path(root, &["features", "fast_mode"]),
        "features_goals": toml_bool_at_path(root, &["features", "goals"]),
        "features_multi_agent": toml_bool_at_path(root, &["features", "multi_agent"]),
        "web_search": root.get("web_search").and_then(|value| value.as_str()),
        "tui_status_line": toml_string_array_at_path(root, &["tui", "status_line"]),
        "model_instructions_file": root
            .get("model_instructions_file")
            .and_then(|value| value.as_str()),
    });
    write_json_value(&path, &value);
}

pub(super) fn overlay_is_empty(overlay: &super::CodexRuntimeOverlay<'_>) -> bool {
    !overlay.yolo_enabled
        && !overlay.fast_enabled
        && !overlay.goals_enabled
        && !overlay.multi_agent_enabled
        && overlay.web_search_mode == "default"
        && overlay.status_line_items.is_empty()
        && !overlay.jailbreak_prompt_file_enabled
        && !overlay.index_prompt_file_enabled
}
