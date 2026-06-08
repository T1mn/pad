mod fields;
mod prompt;

use super::super::super::common::{
    codex_config_path, codex_permission_state_path, parse_toml_document, serialize_toml_document,
    write_text_file,
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

    write_text_file(&path, &serialize_toml_document(&doc));

    if overlay_is_empty(&overlay) {
        let _ = std::fs::remove_file(codex_permission_state_path());
    }
}
