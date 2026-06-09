use super::provider_list_subtitle;
use crate::theme::{OpenCodeModelConfig, ProviderConfig};

fn provider() -> ProviderConfig {
    ProviderConfig {
        label: "OpenAI".into(),
        base_url: "https://api.example.com/v1".into(),
        api_key: "sk-test".into(),
        env_key: String::new(),
        wire_api: String::new(),
        provider_key: "openai".into(),
        npm_package: "@ai-sdk/openai-compatible".into(),
        models: vec![OpenCodeModelConfig {
            id: "gpt-5".into(),
            name: "GPT-5".into(),
        }],
        test_status: Some(true),
        test_http_status: None,
        test_latency_ms: None,
        test_result: None,
    }
}

#[test]
fn provider_subtitle_keeps_opencode_summary_shape() {
    assert_eq!(
        provider_list_subtitle("opencode", &provider()),
        "1 models  |  @ai-sdk/openai-compat...  |  https://api.example.com/v1  |  probe ok"
    );
}

#[test]
fn provider_subtitle_falls_back_when_empty() {
    let mut provider = provider();
    provider.base_url.clear();
    provider.test_status = None;

    assert_eq!(provider_list_subtitle("claude", &provider), "-");
}
