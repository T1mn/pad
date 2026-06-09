pub(super) fn codex_probe_model() -> String {
    std::env::var("PAD_CODEX_PROVIDER_TEST_MODEL")
        .ok()
        .filter(|model| !model.trim().is_empty())
        .or_else(read_configured_codex_model)
        .unwrap_or_else(|| "gpt-5.5".to_string())
}

fn read_configured_codex_model() -> Option<String> {
    let content = std::fs::read_to_string(crate::paths::pad_codex_config_path()).ok()?;
    let parsed = content.parse::<toml::Value>().ok()?;
    parsed
        .get("model")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(ToOwned::to_owned)
}
