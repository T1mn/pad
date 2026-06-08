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
        assert!(value.get("mcpServers").is_some());
        assert!(value.get("apiUrl").is_none());
        assert!(value.get("apiKey").is_none());
    });
}
