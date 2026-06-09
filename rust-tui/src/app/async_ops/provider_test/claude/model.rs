pub(super) fn claude_probe_model(configured_model: &str) -> String {
    std::env::var("PAD_CLAUDE_PROVIDER_TEST_MODEL")
        .ok()
        .filter(|model| !model.trim().is_empty())
        .or_else(|| {
            let model = configured_model.trim();
            (!model.is_empty()).then(|| model.to_string())
        })
        .unwrap_or_else(|| "claude-sonnet-4-5".to_string())
}
