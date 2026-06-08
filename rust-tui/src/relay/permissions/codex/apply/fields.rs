use super::super::super::toml_helpers::{
    restore_toml_bool_path, restore_toml_string_array_path, restore_toml_string_field,
    set_toml_bool_path, set_toml_string_array_path,
};

pub(super) fn apply_yolo(
    root: &mut toml::map::Map<String, toml::Value>,
    state: &serde_json::Value,
    enabled: bool,
) {
    if enabled {
        root.insert(
            "approval_policy".to_string(),
            toml::Value::String("never".to_string()),
        );
        root.insert(
            "sandbox_mode".to_string(),
            toml::Value::String("danger-full-access".to_string()),
        );
    } else {
        restore_toml_string_field(root, "approval_policy", state.get("approval_policy"));
        restore_toml_string_field(root, "sandbox_mode", state.get("sandbox_mode"));
    }
}

pub(super) fn apply_fast(
    root: &mut toml::map::Map<String, toml::Value>,
    state: &serde_json::Value,
    enabled: bool,
) {
    if enabled {
        root.insert(
            "service_tier".to_string(),
            toml::Value::String("fast".to_string()),
        );
        set_toml_bool_path(root, &["features", "fast_mode"], true);
    } else {
        restore_toml_string_field(root, "service_tier", state.get("service_tier"));
        restore_toml_bool_path(
            root,
            &["features", "fast_mode"],
            state.get("features_fast_mode"),
        );
    }
}

pub(super) fn apply_feature_bool(
    root: &mut toml::map::Map<String, toml::Value>,
    state: &serde_json::Value,
    enabled: bool,
    path: &[&str],
    state_key: &str,
) {
    if enabled {
        set_toml_bool_path(root, path, true);
    } else {
        restore_toml_bool_path(root, path, state.get(state_key));
    }
}

pub(super) fn apply_web_search(
    root: &mut toml::map::Map<String, toml::Value>,
    state: &serde_json::Value,
    mode: &str,
) {
    if mode != "default" {
        root.insert(
            "web_search".to_string(),
            toml::Value::String(mode.to_string()),
        );
    } else {
        restore_toml_string_field(root, "web_search", state.get("web_search"));
    }
}

pub(super) fn apply_status_line(
    root: &mut toml::map::Map<String, toml::Value>,
    state: &serde_json::Value,
    items: &[&str],
) {
    if !items.is_empty() {
        set_toml_string_array_path(root, &["tui", "status_line"], items);
    } else {
        restore_toml_string_array_path(root, &["tui", "status_line"], state.get("tui_status_line"));
    }
}
