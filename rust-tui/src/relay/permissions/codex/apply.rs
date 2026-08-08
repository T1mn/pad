mod fields {
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
            restore_toml_string_array_path(
                root,
                &["tui", "status_line"],
                state.get("tui_status_line"),
            );
        }
    }
}
mod prompt {
    use super::super::super::toml_helpers::restore_toml_string_field;
    use super::super::CodexRuntimeOverlay;

    pub(super) fn apply_prompt_file(
        root: &mut toml::map::Map<String, toml::Value>,
        state: &serde_json::Value,
        overlay: &CodexRuntimeOverlay<'_>,
    ) {
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
    }
}

use super::super::super::common::{
    codex_config_path, codex_permission_state_path, log_file_error, parse_toml_document,
    serialize_toml_document, write_text_file,
};
use super::super::toml_helpers::cleanup_empty_toml_table_path;
use super::state::{
    capture_codex_permission_state_once, overlay_is_empty, read_codex_permission_state,
};
use super::CodexRuntimeOverlay;

pub(in crate::relay::permissions) fn apply_codex_runtime_overlay(overlay: CodexRuntimeOverlay<'_>) {
    let path = codex_config_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = parse_toml_document(&content);
    let root = doc.as_table_mut().expect("root toml value must be a table");

    capture_codex_permission_state_once(root);
    let state = read_codex_permission_state();

    fields::apply_yolo(root, &state, overlay.yolo_enabled);
    fields::apply_fast(root, &state, overlay.fast_enabled);
    fields::apply_feature_bool(
        root,
        &state,
        overlay.goals_enabled,
        &["features", "goals"],
        "features_goals",
    );
    fields::apply_feature_bool(
        root,
        &state,
        overlay.multi_agent_enabled,
        &["features", "multi_agent"],
        "features_multi_agent",
    );
    fields::apply_web_search(root, &state, overlay.web_search_mode);
    fields::apply_status_line(root, &state, overlay.status_line_items);
    prompt::apply_prompt_file(root, &state, &overlay);

    cleanup_empty_toml_table_path(root, &["features"]);
    cleanup_empty_toml_table_path(root, &["tui"]);

    if let Err(error) = write_text_file(&path, &serialize_toml_document(&doc)) {
        log_file_error("write", &path, &error);
    }

    if overlay_is_empty(&overlay) {
        let _ = std::fs::remove_file(codex_permission_state_path());
    }
}
