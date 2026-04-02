use super::common::{
    claude_permission_state_path, claude_settings_path, codex_config_path,
    codex_permission_state_path, parse_json_object, parse_toml_document, read_json_value,
    serialize_json_pretty, serialize_toml_document, write_json_value, write_text_file,
};
use crate::theme::{AgentConfig, AgentPermissionsConfig};
use serde_json::json;

pub(super) fn apply_permission_overlays(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
) {
    let has_codex = agents.iter().any(|agent| agent.name == "codex");
    let has_claude = agents.iter().any(|agent| agent.name == "claude");

    if has_codex && permissions.codex_auto_full_access {
        apply_codex_permission_overlay();
    } else if has_codex {
        remove_codex_permission_overlay();
    }

    if has_claude && permissions.claude_auto_full_access {
        apply_claude_permission_overlay();
    } else if has_claude {
        remove_claude_permission_overlay();
    }
}

fn apply_codex_permission_overlay() {
    let path = codex_config_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = parse_toml_document(&content);
    let root = doc.as_table_mut().expect("root toml value must be a table");

    capture_codex_permission_state_once(root);

    root.insert(
        "approval_policy".to_string(),
        toml::Value::String("never".to_string()),
    );
    root.insert(
        "sandbox_mode".to_string(),
        toml::Value::String("danger-full-access".to_string()),
    );

    write_text_file(&path, &serialize_toml_document(&doc));
}

fn remove_codex_permission_overlay() {
    let path = codex_config_path();
    let state_path = codex_permission_state_path();
    if !path.exists() && !state_path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = parse_toml_document(&content);
    let root = doc.as_table_mut().expect("root toml value must be a table");
    let state = read_json_value(
        &state_path,
        json!({ "approval_policy": null, "sandbox_mode": null }),
    );

    restore_toml_string_field(root, "approval_policy", state.get("approval_policy"));
    restore_toml_string_field(root, "sandbox_mode", state.get("sandbox_mode"));

    write_text_file(&path, &serialize_toml_document(&doc));
    let _ = std::fs::remove_file(state_path);
}

fn capture_codex_permission_state_once(root: &toml::map::Map<String, toml::Value>) {
    let path = codex_permission_state_path();
    if path.exists() {
        return;
    }

    let value = json!({
        "approval_policy": root.get("approval_policy").and_then(|value| value.as_str()),
        "sandbox_mode": root.get("sandbox_mode").and_then(|value| value.as_str()),
    });
    write_json_value(&path, &value);
}

fn restore_toml_string_field(
    root: &mut toml::map::Map<String, toml::Value>,
    key: &str,
    previous: Option<&serde_json::Value>,
) {
    if let Some(previous) = previous.and_then(|value| value.as_str()) {
        root.insert(key.to_string(), toml::Value::String(previous.to_string()));
    } else {
        root.remove(key);
    }
}

fn apply_claude_permission_overlay() {
    let path = claude_settings_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut obj = parse_json_object(&content);

    capture_claude_permission_state_once(&obj);
    set_json_string_path(
        &mut obj,
        &["permissions", "defaultMode"],
        "bypassPermissions",
    );
    set_json_bool_path(&mut obj, &["sandbox", "enabled"], false);

    write_text_file(&path, &serialize_json_pretty(&obj));
}

fn remove_claude_permission_overlay() {
    let path = claude_settings_path();
    let state_path = claude_permission_state_path();
    if !path.exists() && !state_path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut obj = parse_json_object(&content);
    let state = read_json_value(
        &state_path,
        json!({
            "permissions_default_mode": null,
            "sandbox_enabled": null
        }),
    );

    restore_json_string_path(
        &mut obj,
        &["permissions", "defaultMode"],
        state.get("permissions_default_mode"),
    );
    restore_json_bool_path(
        &mut obj,
        &["sandbox", "enabled"],
        state.get("sandbox_enabled"),
    );
    cleanup_empty_json_objects(&mut obj);

    write_text_file(&path, &serialize_json_pretty(&obj));
    let _ = std::fs::remove_file(state_path);
}

fn capture_claude_permission_state_once(obj: &serde_json::Value) {
    let path = claude_permission_state_path();
    if path.exists() {
        return;
    }

    let value = json!({
        "permissions_default_mode": json_string_at_path(obj, &["permissions", "defaultMode"]),
        "sandbox_enabled": json_bool_at_path(obj, &["sandbox", "enabled"]),
    });
    write_json_value(&path, &value);
}

fn json_string_at_path(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

fn json_bool_at_path(value: &serde_json::Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}

fn set_json_string_path(value: &mut serde_json::Value, path: &[&str], text: &str) {
    let mut current = value;
    for key in &path[..path.len().saturating_sub(1)] {
        current = ensure_json_child_value(current, key);
    }
    if let Some(last) = path.last() {
        current.as_object_mut().expect("json object").insert(
            (*last).to_string(),
            serde_json::Value::String(text.to_string()),
        );
    }
}

fn set_json_bool_path(value: &mut serde_json::Value, path: &[&str], flag: bool) {
    let mut current = value;
    for key in &path[..path.len().saturating_sub(1)] {
        current = ensure_json_child_value(current, key);
    }
    if let Some(last) = path.last() {
        current
            .as_object_mut()
            .expect("json object")
            .insert((*last).to_string(), serde_json::Value::Bool(flag));
    }
}

fn ensure_json_child_value<'a>(
    value: &'a mut serde_json::Value,
    key: &str,
) -> &'a mut serde_json::Value {
    let object = value.as_object_mut().expect("json object");
    let entry = object.entry(key.to_string()).or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    entry
}

fn restore_json_string_path(
    value: &mut serde_json::Value,
    path: &[&str],
    previous: Option<&serde_json::Value>,
) {
    if let Some(previous) = previous.and_then(|value| value.as_str()) {
        set_json_string_path(value, path, previous);
    } else {
        remove_json_path(value, path);
    }
}

fn restore_json_bool_path(
    value: &mut serde_json::Value,
    path: &[&str],
    previous: Option<&serde_json::Value>,
) {
    if let Some(previous) = previous.and_then(|value| value.as_bool()) {
        set_json_bool_path(value, path, previous);
    } else {
        remove_json_path(value, path);
    }
}

fn remove_json_path(value: &mut serde_json::Value, path: &[&str]) {
    if path.is_empty() {
        return;
    }
    let Some(root) = value.as_object_mut() else {
        return;
    };
    remove_json_path_in_map(root, path);
}

fn remove_json_path_in_map(
    map: &mut serde_json::Map<String, serde_json::Value>,
    path: &[&str],
) -> bool {
    if path.len() == 1 {
        map.remove(path[0]);
        return map.is_empty();
    }

    let remove_child = if let Some(child) = map.get_mut(path[0]) {
        if let Some(child_map) = child.as_object_mut() {
            remove_json_path_in_map(child_map, &path[1..])
        } else {
            false
        }
    } else {
        false
    };

    if remove_child {
        map.remove(path[0]);
    }

    map.is_empty()
}

fn cleanup_empty_json_objects(value: &mut serde_json::Value) -> bool {
    let Some(map) = value.as_object_mut() else {
        return false;
    };

    let keys = map.keys().cloned().collect::<Vec<_>>();
    for key in keys {
        let remove_key = map
            .get_mut(&key)
            .map(cleanup_empty_json_objects)
            .unwrap_or(false);
        if remove_key {
            map.remove(&key);
        }
    }

    map.is_empty()
}
