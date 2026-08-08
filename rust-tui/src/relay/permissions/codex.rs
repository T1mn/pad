mod apply;
mod remove {
    use super::super::super::common::{
        codex_config_path, codex_permission_state_path, log_file_error, parse_toml_document,
        serialize_toml_document, write_text_file,
    };
    use super::super::toml_helpers::{
        cleanup_empty_toml_table_path, restore_toml_bool_path, restore_toml_string_array_path,
        restore_toml_string_field,
    };
    use super::state::{overlay_is_empty, read_codex_permission_state};
    use super::CodexRuntimeOverlay;

    pub(in crate::relay::permissions) fn remove_codex_runtime_overlay(
        overlay: CodexRuntimeOverlay<'_>,
    ) {
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
            restore_toml_string_array_path(
                root,
                &["tui", "status_line"],
                state.get("tui_status_line"),
            );
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

        if let Err(error) = write_text_file(&path, &serialize_toml_document(&doc)) {
            log_file_error("write", &path, &error);
        }
        if overlay_is_empty(&overlay) {
            let _ = std::fs::remove_file(state_path);
        }
    }
}
mod state {
    use super::super::super::common::{
        codex_permission_state_path, log_file_error, read_json_value, write_json_value,
    };
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
        if let Err(error) = write_json_value(&path, &value) {
            log_file_error("write", &path, &error);
        }
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
}

pub(super) use apply::apply_codex_runtime_overlay;
pub(super) use remove::remove_codex_runtime_overlay;

pub(super) struct CodexRuntimeOverlay<'a> {
    pub(super) yolo_enabled: bool,
    pub(super) fast_enabled: bool,
    pub(super) goals_enabled: bool,
    pub(super) multi_agent_enabled: bool,
    pub(super) web_search_mode: &'a str,
    pub(super) status_line_items: &'a [&'a str],
    pub(super) jailbreak_prompt_file_enabled: bool,
    pub(super) index_prompt_file_enabled: bool,
}
