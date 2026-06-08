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
