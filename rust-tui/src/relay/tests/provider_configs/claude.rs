#[test]
fn claude_provider_writes_cc_switch_style_env_settings() {
    with_temp_home("claude-write", |home| {
        let settings_path = home.join(".claude").join("settings.json");
        std::fs::create_dir_all(settings_path.parent().expect("claude dir"))
            .expect("create claude dir");
        std::fs::write(
            &settings_path,
            r#"{"mcpServers":{"echo":{"command":"echo"}},"apiUrl":"old","apiKey":"old"}"#,
        )
        .expect("seed claude settings");

        let agent = AgentConfig {
            name: "claude".into(),
            cmd: "claude".into(),
            providers: vec![sample_provider(
                "https://claude-relay.example",
                "sk-ant-test",
            )],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        };
        apply_relay_configs(&[agent]);

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert_eq!(
            value
                .pointer("/env/ANTHROPIC_BASE_URL")
                .and_then(|v| v.as_str()),
            Some("https://claude-relay.example")
        );
        assert_eq!(
            value
                .pointer("/env/ANTHROPIC_AUTH_TOKEN")
                .and_then(|v| v.as_str()),
            Some("sk-ant-test")
        );
        assert_eq!(
            value
                .pointer("/env/CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC")
                .and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            value
                .pointer("/env/CLAUDE_CODE_ATTRIBUTION_HEADER")
                .and_then(|v| v.as_str()),
            Some("0")
        );
        assert!(value.get("mcpServers").is_some());
        assert!(value.get("apiUrl").is_none());
        assert!(value.get("apiKey").is_none());
    });
}

#[test]
fn claude_provider_strips_trailing_v1_from_base_url() {
    let updated = crate::relay::claude::update_claude_settings_config(
        "{}",
        "https://claude-relay.example/v1/",
        "sk-ant-test",
        "",
    );
    let value: serde_json::Value = serde_json::from_str(&updated).expect("parse");

    assert_eq!(
        value
            .pointer("/env/ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str()),
        Some("https://claude-relay.example")
    );
}

#[test]
fn claude_provider_writes_default_model_env_when_configured() {
    let updated = crate::relay::claude::update_claude_settings_config(
        "{}",
        "https://claude-relay.example",
        "sk-ant-test",
        "claude-sonnet-4-5",
    );
    let value: serde_json::Value = serde_json::from_str(&updated).expect("parse");

    assert_eq!(
        value.pointer("/env/ANTHROPIC_MODEL").and_then(|v| v.as_str()),
        Some("claude-sonnet-4-5")
    );
    assert_eq!(
        value
            .pointer("/env/ANTHROPIC_CUSTOM_MODEL_OPTION")
            .and_then(|v| v.as_str()),
        Some("claude-sonnet-4-5")
    );
}
