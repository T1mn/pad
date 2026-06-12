pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
    crate::test_support::with_temp_home("pad-relay-reload", name, |_| f())
}

pub(super) fn sample_provider(label: &str, base_url: &str, api_key: &str) -> ProviderConfig {
    ProviderConfig {
        label: label.to_string(),
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        env_key: String::new(),
        wire_api: "responses".to_string(),
        provider_key: crate::theme::normalize_provider_key(label),
        npm_package: "@ai-sdk/openai-compatible".to_string(),
        disable_thinking: false,
        models: Vec::new(),
        test_status: Some(true),
        test_http_status: Some(200),
        test_latency_ms: Some(12),
        test_result: Some("ok".to_string()),
    }
}

pub(super) fn seed_codex_provider(label: &str, base_url: &str, api_key: &str) {
    let mut config = Config::default();
    let codex = config
        .agents
        .iter_mut()
        .find(|agent| agent.name == "codex")
        .expect("codex agent");
    codex.providers = vec![sample_provider(label, base_url, api_key)];
    codex.active_provider = Some(0);
    config.save();
}

pub(super) fn update_codex_provider(label: &str, base_url: &str, api_key: &str) {
    let mut updated = Config::load();
    let codex = updated
        .agents
        .iter_mut()
        .find(|agent| agent.name == "codex")
        .expect("updated codex agent");
    codex.providers = vec![sample_provider(label, base_url, api_key)];
    codex.active_provider = Some(0);
    updated.save();
}

pub(super) fn current_codex_provider(app: &App) -> &ProviderConfig {
    app.config
        .agents
        .iter()
        .find(|agent| agent.name == "codex")
        .expect("current codex agent")
        .providers
        .first()
        .expect("codex provider")
}

pub(super) fn poll_external_relay_config(app: &mut App) {
    app.relay_config_last_poll_at -= RELAY_CONFIG_POLL_INTERVAL;
    app.poll_external_relay_config_if_due();
}
