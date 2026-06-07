use super::common::{
    claude_permission_state_path, claude_settings_path, codex_config_path,
    codex_permission_state_path, parse_json_object, parse_toml_document, read_json_value,
    serialize_json_pretty, serialize_toml_document, write_json_value, write_text_file,
};
use crate::theme::{AgentConfig, AgentPermissionsConfig, CodexConfig};
use serde_json::json;

mod json_helpers;
mod toml_helpers;

use json_helpers::{
    cleanup_empty_json_objects, json_bool_at_path, json_string_at_path, restore_json_bool_path,
    restore_json_string_path, set_json_bool_path, set_json_string_path,
};
use toml_helpers::{
    cleanup_empty_toml_table_path, restore_toml_bool_path, restore_toml_string_array_path,
    restore_toml_string_field, set_toml_bool_path, set_toml_string_array_path, toml_bool_at_path,
    toml_string_array_at_path,
};

struct CodexRuntimeOverlay<'a> {
    yolo_enabled: bool,
    fast_enabled: bool,
    goals_enabled: bool,
    multi_agent_enabled: bool,
    web_search_mode: &'a str,
    status_line_items: &'a [&'a str],
    jailbreak_prompt_file_enabled: bool,
    index_prompt_file_enabled: bool,
}

pub(super) fn apply_runtime_overlays(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
    codex: &CodexConfig,
) {
    let has_codex = agents.iter().any(|agent| agent.name == "codex");
    let has_claude = agents.iter().any(|agent| agent.name == "claude");

    if has_codex {
        let status_line_items = codex.status_line_items();
        apply_codex_runtime_overlay(CodexRuntimeOverlay {
            yolo_enabled: permissions.codex_auto_full_access,
            fast_enabled: codex.fast_mode,
            goals_enabled: codex.goals,
            multi_agent_enabled: codex.multi_agent,
            web_search_mode: &codex.web_search,
            status_line_items: &status_line_items,
            jailbreak_prompt_file_enabled: codex.jailbreak_prompt_file,
            index_prompt_file_enabled: codex.index_prompt_file,
        });
    } else {
        remove_codex_runtime_overlay(CodexRuntimeOverlay {
            yolo_enabled: false,
            fast_enabled: false,
            goals_enabled: false,
            multi_agent_enabled: false,
            web_search_mode: "default",
            status_line_items: &[],
            jailbreak_prompt_file_enabled: false,
            index_prompt_file_enabled: false,
        });
    }

    if has_claude && permissions.claude_auto_full_access {
        apply_claude_permission_overlay();
    } else if has_claude {
        remove_claude_permission_overlay();
    }
}

fn apply_codex_runtime_overlay(overlay: CodexRuntimeOverlay<'_>) {
    let path = codex_config_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = parse_toml_document(&content);
    let root = doc.as_table_mut().expect("root toml value must be a table");

    capture_codex_permission_state_once(root);
    let state = read_codex_permission_state();

    if overlay.yolo_enabled {
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

    if overlay.fast_enabled {
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

    if overlay.goals_enabled {
        set_toml_bool_path(root, &["features", "goals"], true);
    } else {
        restore_toml_bool_path(root, &["features", "goals"], state.get("features_goals"));
    }

    if overlay.multi_agent_enabled {
        set_toml_bool_path(root, &["features", "multi_agent"], true);
    } else {
        restore_toml_bool_path(
            root,
            &["features", "multi_agent"],
            state.get("features_multi_agent"),
        );
    }

    if overlay.web_search_mode != "default" {
        root.insert(
            "web_search".to_string(),
            toml::Value::String(overlay.web_search_mode.to_string()),
        );
    } else {
        restore_toml_string_field(root, "web_search", state.get("web_search"));
    }

    if !overlay.status_line_items.is_empty() {
        set_toml_string_array_path(root, &["tui", "status_line"], overlay.status_line_items);
    } else {
        restore_toml_string_array_path(root, &["tui", "status_line"], state.get("tui_status_line"));
    }

    if let Ok(Some(prompt_path)) = crate::paths::write_codex_selected_prompt_file(
        overlay.jailbreak_prompt_file_enabled,
        overlay.index_prompt_file_enabled,
    ) {
        root.insert(
            "model_instructions_file".to_string(),
            toml::Value::String(prompt_path.to_string_lossy().to_string()),
        );
    } else {
        restore_toml_string_field(
            root,
            "model_instructions_file",
            state.get("model_instructions_file"),
        );
    }

    cleanup_empty_toml_table_path(root, &["features"]);
    cleanup_empty_toml_table_path(root, &["tui"]);

    write_text_file(&path, &serialize_toml_document(&doc));

    if !overlay.yolo_enabled
        && !overlay.fast_enabled
        && !overlay.goals_enabled
        && !overlay.multi_agent_enabled
        && overlay.web_search_mode == "default"
        && overlay.status_line_items.is_empty()
        && !overlay.jailbreak_prompt_file_enabled
        && !overlay.index_prompt_file_enabled
    {
        let _ = std::fs::remove_file(codex_permission_state_path());
    }
}

fn read_codex_permission_state() -> serde_json::Value {
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

fn remove_codex_runtime_overlay(overlay: CodexRuntimeOverlay<'_>) {
    let path = codex_config_path();
    let state_path = codex_permission_state_path();
    if !path.exists() && !state_path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = parse_toml_document(&content);
    let root = doc.as_table_mut().expect("root toml value must be a table");
    let state = read_codex_permission_state();

    if !overlay.yolo_enabled {
        restore_toml_string_field(root, "approval_policy", state.get("approval_policy"));
        restore_toml_string_field(root, "sandbox_mode", state.get("sandbox_mode"));
    }
    if !overlay.fast_enabled {
        restore_toml_string_field(root, "service_tier", state.get("service_tier"));
        restore_toml_bool_path(
            root,
            &["features", "fast_mode"],
            state.get("features_fast_mode"),
        );
    }
    if !overlay.goals_enabled {
        restore_toml_bool_path(root, &["features", "goals"], state.get("features_goals"));
    }
    if !overlay.multi_agent_enabled {
        restore_toml_bool_path(
            root,
            &["features", "multi_agent"],
            state.get("features_multi_agent"),
        );
    }
    if overlay.web_search_mode == "default" {
        restore_toml_string_field(root, "web_search", state.get("web_search"));
    }
    if overlay.status_line_items.is_empty() {
        restore_toml_string_array_path(root, &["tui", "status_line"], state.get("tui_status_line"));
    }
    if !overlay.jailbreak_prompt_file_enabled && !overlay.index_prompt_file_enabled {
        restore_toml_string_field(
            root,
            "model_instructions_file",
            state.get("model_instructions_file"),
        );
    }
    cleanup_empty_toml_table_path(root, &["features"]);
    cleanup_empty_toml_table_path(root, &["tui"]);

    write_text_file(&path, &serialize_toml_document(&doc));
    if !overlay.yolo_enabled
        && !overlay.fast_enabled
        && !overlay.goals_enabled
        && !overlay.multi_agent_enabled
        && overlay.web_search_mode == "default"
        && overlay.status_line_items.is_empty()
        && !overlay.jailbreak_prompt_file_enabled
        && !overlay.index_prompt_file_enabled
    {
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
