#[test]
fn runtime_overlays_do_not_rewrite_opencode_live_provider_config() {
    with_temp_home("opencode-overlay-only", |home| {
        let config_path = home.join(".config").join("opencode").join("opencode.json");
        std::fs::create_dir_all(config_path.parent().expect("opencode dir"))
            .expect("create opencode dir");
        let seed = r#"{"$schema":"https://opencode.ai/config.json","provider":{"relay":{"npm":"@ai-sdk/openai-compatible","name":"cpa","options":{"baseURL":"https://cpa.example/v1","apiKey":"sk-live"},"models":{"grok-4.5":{"name":"grok-4.5"}}}},"model":"relay/grok-4.5"}"#;
        std::fs::write(&config_path, seed).expect("seed opencode config");
        std::fs::create_dir_all(opencode_managed_state_path().parent().expect("pad home"))
            .expect("pad home");
        std::fs::write(
            opencode_managed_state_path(),
            r#"{"provider_keys":["relay"]}"#,
        )
        .expect("seed managed state");

        let agent = AgentConfig {
            name: "opencode".into(),
            cmd: "opencode".into(),
            providers: vec![ProviderConfig {
                label: "relay".into(),
                base_url: "https://example.test".into(),
                api_key: "sk-stale".into(),
                env_key: String::new(),
                wire_api: "responses".into(),
                provider_key: "relay".into(),
                npm_package: "@ai-sdk/openai-compatible".into(),
                disable_thinking: false,
                models: Vec::new(),
                test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            }],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        };

        apply_runtime_overlays(&[agent], &sample_permissions(), &sample_codex_config());

        let after = std::fs::read_to_string(&config_path).expect("read");
        assert_eq!(after, seed);
    });
}
