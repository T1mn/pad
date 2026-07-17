fn opencode_agent() -> AgentConfig {
    AgentConfig {
        name: "opencode".into(),
        cmd: "opencode".into(),
        providers: vec![ProviderConfig {
            label: "Relay A".into(),
            base_url: "https://relay.example/v1".into(),
            api_key: "sk-op-test".into(),
            env_key: String::new(),
            wire_api: "responses".into(),
            provider_key: "relay-a".into(),
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
    }
}

#[test]
fn opencode_provider_does_not_overwrite_malformed_config() {
    with_temp_home("opencode-malformed", |home| {
        let config_path = home.join(".config/opencode/opencode.json");
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        let original = r#"{"theme":"tokyonight","provider":{"external":}}"#;
        std::fs::write(&config_path, original).unwrap();
        apply_relay_configs(&[opencode_agent()]);
        assert_eq!(std::fs::read_to_string(config_path).unwrap(), original);
    });
}

#[test]
fn opencode_provider_does_not_overwrite_unsupported_jsonc() {
    with_temp_home("opencode-unsupported-jsonc", |home| {
        let config_path = home.join(".config/opencode/opencode.jsonc");
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        let original = "{\n  \"theme\": \"tokyonight\",\n}\n";
        std::fs::write(&config_path, original).unwrap();
        apply_relay_configs(&[opencode_agent()]);
        assert_eq!(std::fs::read_to_string(config_path).unwrap(), original);
    });
}
