use super::super::super::common::{
    codex_config_path, codex_permission_state_path, parse_toml_document, serialize_toml_document,
    write_text_file,
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
    if overlay_is_empty(&overlay) {
        let _ = std::fs::remove_file(state_path);
    }
}
