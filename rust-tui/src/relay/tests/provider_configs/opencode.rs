#[test]
fn opencode_provider_prefers_existing_jsonc_and_preserves_urls_in_strings() {
    with_temp_home("opencode-jsonc", |home| {
        let config_dir = home.join(".config").join("opencode");
        std::fs::create_dir_all(&config_dir).expect("create opencode dir");
        let config_path = config_dir.join("opencode.jsonc");
        std::fs::write(
            &config_path,
            r#"{
              // OpenCode accepts JSONC config files.
              "$schema": "https://opencode.ai/config.json",
              "theme": "tokyonight",
              "note": "keep https://example.test/path intact",
              "provider": {}
            }
            "#,
        )
        .expect("seed jsonc config");

        let agent = AgentConfig {
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
        models: vec![crate::theme::OpenCodeModelConfig {
                    id: "gpt-4o".into(),
                    name: "GPT-4o".into(),
                }],
        test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            }],
            active_provider: Some(0),
            default_model: "relay-a/gpt-4o".into(),
            small_model: String::new(),
        };

        apply_relay_configs(&[agent]);

        assert!(config_path.exists());
        assert!(!config_dir.join("opencode.json").exists());
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).expect("read"))
                .expect("parse");
        assert_eq!(
            value.pointer("/provider/relay-a/options/baseURL").and_then(|v| v.as_str()),
            Some("https://relay.example/v1")
        );
        assert_eq!(
            value.get("note").and_then(|v| v.as_str()),
            Some("keep https://example.test/path intact")
        );
    });
}

#[test]
fn opencode_provider_writes_additive_live_config_and_models() {
    with_temp_home("opencode-write", |home| {
        let config_path = home.join(".config").join("opencode").join("opencode.json");
        std::fs::create_dir_all(config_path.parent().expect("opencode dir"))
            .expect("create opencode dir");
        std::fs::write(
            &config_path,
            r#"{"$schema":"https://opencode.ai/config.json","provider":{"external":{"npm":"@ai-sdk/openai","models":{"gpt-5":{"name":"GPT-5"}}}},"theme":"tokyonight"}"#,
        )
        .expect("seed opencode config");

        let agent = AgentConfig {
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
        models: vec![crate::theme::OpenCodeModelConfig {
                    id: "gpt-4o".into(),
                    name: "GPT-4o".into(),
                }],
        test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            }],
            active_provider: Some(0),
            default_model: "relay-a/gpt-4o".into(),
            small_model: String::new(),
        };

        apply_relay_configs(&[agent]);

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).expect("read"))
                .expect("parse");
        assert_eq!(
            value
                .pointer("/provider/relay-a/options/baseURL")
                .and_then(|v| v.as_str()),
            Some("https://relay.example/v1")
        );
        assert_eq!(
            value
                .pointer("/provider/relay-a/options/apiKey")
                .and_then(|v| v.as_str()),
            Some("sk-op-test")
        );
        assert_eq!(
            value
                .pointer("/provider/relay-a/models/gpt-4o/name")
                .and_then(|v| v.as_str()),
            Some("GPT-4o")
        );
        assert_eq!(
            value.pointer("/model").and_then(|v| v.as_str()),
            Some("relay-a/gpt-4o")
        );
        assert!(value.pointer("/provider/external/models/gpt-5").is_some());
        assert_eq!(
            value.get("theme").and_then(|v| v.as_str()),
            Some("tokyonight")
        );
    });
}

#[test]
fn opencode_sync_removes_previously_managed_provider_keys() {
    with_temp_home("opencode-remove", |home| {
        let config_path = home.join(".config").join("opencode").join("opencode.json");
        std::fs::create_dir_all(config_path.parent().expect("opencode dir"))
            .expect("create opencode dir");
        std::fs::write(
            &config_path,
            r#"{"$schema":"https://opencode.ai/config.json","provider":{"relay-a":{"npm":"@ai-sdk/openai-compatible","models":{"gpt-4o":{"name":"GPT-4o"}}}},"model":"relay-a/gpt-4o"}"#,
        )
        .expect("seed opencode config");
        let managed_state = opencode_managed_state_path();
        std::fs::create_dir_all(managed_state.parent().expect("managed state parent"))
            .expect("pad home");
        std::fs::write(managed_state, r#"{"provider_keys":["relay-a"]}"#)
            .expect("seed managed state");

        let agent = AgentConfig {
            name: "opencode".into(),
            cmd: "opencode".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
        };

        apply_relay_configs(&[agent]);

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).expect("read"))
                .expect("parse");
        assert!(value.pointer("/provider/relay-a").is_none());
        assert!(value.get("model").is_none());
    });
}
