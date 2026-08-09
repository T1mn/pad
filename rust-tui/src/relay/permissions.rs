mod claude {
    use super::super::common::{
        claude_permission_state_path, claude_settings_path, log_file_error, parse_json_object,
        read_json_value, serialize_json_pretty, write_json_value, write_text_file,
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

        if let Err(error) = write_text_file(&path, &serialize_json_pretty(&obj)) {
            log_file_error("write", &path, &error);
        }
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

        if let Err(error) = write_text_file(&path, &serialize_json_pretty(&obj)) {
            log_file_error("write", &path, &error);
        }
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
        if let Err(error) = write_json_value(&path, &value) {
            log_file_error("write", &path, &error);
        }
    }
}
mod codex;
mod json_helpers;
mod toml_helpers;

use crate::theme::{AgentConfig, AgentPermissionsConfig, CodexConfig};
use codex::CodexRuntimeOverlay;

pub(super) fn apply_runtime_overlays(
    agents: &[AgentConfig],
    permissions: &AgentPermissionsConfig,
    codex_config: &CodexConfig,
) {
    let has_codex = agents.iter().any(|agent| agent.name == "codex");
    let has_claude = agents.iter().any(|agent| agent.name == "claude");

    if has_codex {
        let status_line_items = codex_config.status_line_items();
        codex::apply_codex_runtime_overlay(CodexRuntimeOverlay {
            yolo_enabled: permissions.codex_auto_full_access,
            fast_enabled: codex_config.fast_mode,
            goals_enabled: codex_config.goals,
            multi_agent_enabled: codex_config.multi_agent,
            web_search_mode: &codex_config.web_search,
            status_line_items: &status_line_items,
            jailbreak_prompt_file_enabled: codex_config.jailbreak_prompt_file,
            index_prompt_file_enabled: codex_config.index_prompt_file,
        });
    } else {
        codex::remove_codex_runtime_overlay(CodexRuntimeOverlay {
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
        claude::apply_claude_permission_overlay();
    } else if has_claude {
        claude::remove_claude_permission_overlay();
    }
}
