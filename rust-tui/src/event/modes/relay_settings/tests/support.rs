use crate::app::App;
use crate::theme::ProviderConfig;

pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
    crate::test_support::with_temp_home("pad-relay-settings", name, |_| f())
}

pub(super) fn sample_provider(label: &str) -> ProviderConfig {
    ProviderConfig {
        label: label.to_string(),
        base_url: "https://example.test".to_string(),
        api_key: "secret".to_string(),
        env_key: String::new(),
        wire_api: "responses".to_string(),
        provider_key: label.to_string(),
        npm_package: "@ai-sdk/openai-compatible".to_string(),
        models: Vec::new(),
        test_status: None,
        test_http_status: None,
        test_latency_ms: None,
        test_result: None,
    }
}

pub(super) fn agent_index(app: &App, name: &str) -> usize {
    app.config
        .agents
        .iter()
        .position(|agent| agent.name == name)
        .expect("agent")
}
