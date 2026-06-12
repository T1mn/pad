use super::opencode_models_summary;
use crate::theme::{OpenCodeModelConfig, ProviderConfig};

fn provider_with_models(models: Vec<OpenCodeModelConfig>) -> ProviderConfig {
    ProviderConfig {
        label: "relay".into(),
        base_url: String::new(),
        api_key: String::new(),
        env_key: String::new(),
        wire_api: String::new(),
        provider_key: String::new(),
        npm_package: String::new(),
        models,
        disable_thinking: false,
        test_status: None,
        test_http_status: None,
        test_latency_ms: None,
        test_result: None,
    }
}

#[test]
fn opencode_models_summary_formats_first_two_models() {
    let provider = provider_with_models(vec![
        OpenCodeModelConfig {
            id: "gpt-5".into(),
            name: "GPT 5".into(),
        },
        OpenCodeModelConfig {
            id: "mini".into(),
            name: String::new(),
        },
    ]);

    assert_eq!(opencode_models_summary(&provider), "GPT 5 (gpt-5), mini");
}

#[test]
fn opencode_models_summary_counts_remaining_models() {
    let provider = provider_with_models(vec![
        OpenCodeModelConfig {
            id: "a".into(),
            name: "A".into(),
        },
        OpenCodeModelConfig {
            id: "b".into(),
            name: "B".into(),
        },
        OpenCodeModelConfig {
            id: "c".into(),
            name: "C".into(),
        },
    ]);

    assert_eq!(
        opencode_models_summary(&provider),
        "A (a), B (b)  ·  +1 more"
    );
}
