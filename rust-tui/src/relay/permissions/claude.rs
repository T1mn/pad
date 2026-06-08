use super::super::common::{
    claude_permission_state_path, claude_settings_path, parse_json_object, read_json_value,
    serialize_json_pretty, write_json_value, write_text_file,
};
use super::json_helpers::{
    cleanup_empty_json_objects, json_bool_at_path, json_string_at_path, restore_json_bool_path,
    restore_json_string_path, set_json_bool_path, set_json_string_path,
};
use serde_json::json;

pub(super) fn apply_claude_permission_overlay() {
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

pub(super) fn remove_claude_permission_overlay() {
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
