#[test]
fn claude_provider_follows_claude_config_dir() {
    with_temp_home("claude-config-dir", |home| {
        let config_dir = home.join("custom-claude");
        let settings_path = config_dir.join("settings.json");
        std::fs::create_dir_all(&config_dir).expect("create config dir");
        std::fs::write(&settings_path, "{}").expect("seed settings");
        let previous = std::env::var_os("CLAUDE_CONFIG_DIR");
        std::env::set_var("CLAUDE_CONFIG_DIR", &config_dir);
        apply_relay_configs(&[AgentConfig {
            name: "claude".into(),
            cmd: "claude".into(),
            providers: vec![sample_provider(
                "https://claude-relay.example",
                "sk-ant-test",
            )],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        }]);
        if let Some(previous) = previous {
            std::env::set_var("CLAUDE_CONFIG_DIR", previous);
        } else {
            std::env::remove_var("CLAUDE_CONFIG_DIR");
        }
        let value: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(settings_path).expect("read custom settings"),
        )
        .expect("parse custom settings");
        assert_eq!(
            value
                .pointer("/env/ANTHROPIC_BASE_URL")
                .and_then(serde_json::Value::as_str),
            Some("https://claude-relay.example")
        );
        assert!(!home.join(".claude/settings.json").exists());
    });
}

#[test]
fn claude_provider_does_not_overwrite_malformed_settings() {
    with_temp_home("claude-malformed", |home| {
        let settings_path = home.join(".claude/settings.json");
        std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        let original = r#"{"env":{"KEEP":"yes",}}"#;
        std::fs::write(&settings_path, original).unwrap();
        apply_relay_configs(&[AgentConfig {
            name: "claude".into(),
            cmd: "claude".into(),
            providers: vec![sample_provider(
                "https://claude-relay.example",
                "sk-ant-test",
            )],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        }]);
        assert_eq!(std::fs::read_to_string(settings_path).unwrap(), original);
    });
}
