use super::common::{
    claude_permission_state_path, claude_settings_path, codex_config_path,
    codex_permission_state_path, parse_json_object, parse_toml_document, read_json_value,
    serialize_json_pretty, serialize_toml_document, write_json_value, write_text_file,
};
use crate::theme::{AgentConfig, AgentPermissionsConfig, CodexConfig};
use serde_json::json;

pub(super) fn apply_runtime_overlays(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
    codex: &CodexConfig,
) {
    let has_codex = agents.iter().any(|agent| agent.name == "codex");
    let has_claude = agents.iter().any(|agent| agent.name == "claude");

    if has_codex {
        apply_codex_runtime_overlay(
            permissions.codex_auto_full_access,
            codex.fast_mode,
            codex.multi_agent,
            &codex.web_search,
        );
    } else {
        remove_codex_runtime_overlay(false, false, false, "default");
    }

    if has_claude && permissions.claude_auto_full_access {
        apply_claude_permission_overlay();
    } else if has_claude {
        remove_claude_permission_overlay();
    }
}

fn apply_codex_runtime_overlay(
    yolo_enabled: bool,
    fast_enabled: bool,
    multi_agent_enabled: bool,
    web_search_mode: &str,
) {
    let path = codex_config_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = parse_toml_document(&content);
    let root = doc.as_table_mut().expect("root toml value must be a table");

    capture_codex_permission_state_once(root);

    if yolo_enabled {
        root.insert(
            "approval_policy".to_string(),
            toml::Value::String("never".to_string()),
        );
        root.insert(
            "sandbox_mode".to_string(),
            toml::Value::String("danger-full-access".to_string()),
        );
    } else {
        let state = read_json_value(
            &codex_permission_state_path(),
            json!({
                "approval_policy": null,
                "sandbox_mode": null,
                "service_tier": null,
                "features_fast_mode": null,
                "features_multi_agent": null,
                "web_search": null
            }),
        );
        restore_toml_string_field(root, "approval_policy", state.get("approval_policy"));
        restore_toml_string_field(root, "sandbox_mode", state.get("sandbox_mode"));
    }

    if fast_enabled {
        root.insert(
            "service_tier".to_string(),
            toml::Value::String("fast".to_string()),
        );
        set_toml_bool_path(root, &["features", "fast_mode"], true);
    } else {
        let state = read_json_value(
            &codex_permission_state_path(),
            json!({
                "approval_policy": null,
                "sandbox_mode": null,
                "service_tier": null,
                "features_fast_mode": null,
                "features_multi_agent": null,
                "web_search": null
            }),
        );
        restore_toml_string_field(root, "service_tier", state.get("service_tier"));
        restore_toml_bool_path(
            root,
            &["features", "fast_mode"],
            state.get("features_fast_mode"),
        );
    }

    if multi_agent_enabled {
        set_toml_bool_path(root, &["features", "multi_agent"], true);
    } else {
        let state = read_json_value(
            &codex_permission_state_path(),
            json!({
                "approval_policy": null,
                "sandbox_mode": null,
                "service_tier": null,
                "features_fast_mode": null,
                "features_multi_agent": null,
                "web_search": null
            }),
        );
        restore_toml_bool_path(
            root,
            &["features", "multi_agent"],
            state.get("features_multi_agent"),
        );
    }

    if web_search_mode != "default" {
        root.insert(
            "web_search".to_string(),
            toml::Value::String(web_search_mode.to_string()),
        );
    } else {
        let state = read_json_value(
            &codex_permission_state_path(),
            json!({
                "approval_policy": null,
                "sandbox_mode": null,
                "service_tier": null,
                "features_fast_mode": null,
                "features_multi_agent": null,
                "web_search": null
            }),
        );
        restore_toml_string_field(root, "web_search", state.get("web_search"));
    }

    cleanup_empty_toml_table_path(root, &["features"]);

    write_text_file(&path, &serialize_toml_document(&doc));

    if !yolo_enabled && !fast_enabled && !multi_agent_enabled && web_search_mode == "default" {
        let _ = std::fs::remove_file(codex_permission_state_path());
    }
}

fn remove_codex_runtime_overlay(
    yolo_enabled: bool,
    fast_enabled: bool,
    multi_agent_enabled: bool,
    web_search_mode: &str,
) {
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
        json!({
            "approval_policy": null,
            "sandbox_mode": null,
            "service_tier": null,
            "features_fast_mode": null,
            "features_multi_agent": null,
            "web_search": null
        }),
    );

    if !yolo_enabled {
        restore_toml_string_field(root, "approval_policy", state.get("approval_policy"));
        restore_toml_string_field(root, "sandbox_mode", state.get("sandbox_mode"));
    }
    if !fast_enabled {
        restore_toml_string_field(root, "service_tier", state.get("service_tier"));
        restore_toml_bool_path(
            root,
            &["features", "fast_mode"],
            state.get("features_fast_mode"),
        );
    }
    if !multi_agent_enabled {
        restore_toml_bool_path(
            root,
            &["features", "multi_agent"],
            state.get("features_multi_agent"),
        );
    }
    if web_search_mode == "default" {
        restore_toml_string_field(root, "web_search", state.get("web_search"));
    }
    cleanup_empty_toml_table_path(root, &["features"]);

    write_text_file(&path, &serialize_toml_document(&doc));
    if !yolo_enabled && !fast_enabled && !multi_agent_enabled && web_search_mode == "default" {
        let _ = std::fs::remove_file(state_path);
    }
}

fn capture_codex_permission_state_once(root: &toml::map::Map<String, toml::Value>) {
    let path = codex_permission_state_path();
    if path.exists() {
        return;
    }

    let value = json!({
        "approval_policy": root.get("approval_policy").and_then(|value| value.as_str()),
        "sandbox_mode": root.get("sandbox_mode").and_then(|value| value.as_str()),
        "service_tier": root.get("service_tier").and_then(|value| value.as_str()),
        "features_fast_mode": toml_bool_at_path(root, &["features", "fast_mode"]),
        "features_multi_agent": toml_bool_at_path(root, &["features", "multi_agent"]),
        "web_search": root.get("web_search").and_then(|value| value.as_str()),
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

fn set_toml_bool_path(root: &mut toml::map::Map<String, toml::Value>, path: &[&str], flag: bool) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let mut current = root;
    for key in parents {
        let entry = current
            .entry((*key).to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
        if !entry.is_table() {
            *entry = toml::Value::Table(toml::map::Map::new());
        }
        current = entry.as_table_mut().expect("nested toml table");
    }

    current.insert((*last).to_string(), toml::Value::Boolean(flag));
}

fn restore_toml_bool_path(
    root: &mut toml::map::Map<String, toml::Value>,
    path: &[&str],
    previous: Option<&serde_json::Value>,
) {
    if let Some(previous) = previous.and_then(|value| value.as_bool()) {
        set_toml_bool_path(root, path, previous);
    } else {
        remove_toml_path(root, path);
    }
}

fn toml_bool_at_path(root: &toml::map::Map<String, toml::Value>, path: &[&str]) -> Option<bool> {
    let mut current = root.get(*path.first()?)?;
    for key in &path[1..] {
        current = current.as_table()?.get(*key)?;
    }
    current.as_bool()
}

fn remove_toml_path(root: &mut toml::map::Map<String, toml::Value>, path: &[&str]) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let mut current = root;
    for key in parents {
        let Some(next) = current.get_mut(*key) else {
            return;
        };
        let Some(next_table) = next.as_table_mut() else {
            return;
        };
        current = next_table;
    }
    current.remove(*last);
}

fn cleanup_empty_toml_table_path(root: &mut toml::map::Map<String, toml::Value>, path: &[&str]) {
    if path.is_empty() {
        return;
    }

    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let mut current = root;
    for key in parents {
        let Some(next) = current.get_mut(*key) else {
            return;
        };
        let Some(next_table) = next.as_table_mut() else {
            return;
        };
        current = next_table;
    }

    let should_remove = current
        .get(*last)
        .and_then(|value| value.as_table())
        .map(|table| table.is_empty())
        .unwrap_or(false);
    if should_remove {
        current.remove(*last);
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
