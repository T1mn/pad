#[test]
fn incomplete_codex_provider_restores_native_config() {
    let agent = AgentConfig {
        name: "codex".into(),
        cmd: "codex".into(),
        providers: vec![sample_provider("http://relay.example", "")],
        active_provider: Some(0),
        default_model: String::new(),
        small_model: String::new(),
    };

    assert!(should_restore_native_codex_config(&agent));
}

#[test]
fn complete_codex_provider_keeps_relay_config() {
    let agent = AgentConfig {
        name: "codex".into(),
        cmd: "codex".into(),
        providers: vec![sample_provider("http://relay.example", "sk-test")],
        active_provider: Some(0),
        default_model: String::new(),
        small_model: String::new(),
    };

    assert!(!should_restore_native_codex_config(&agent));
}

#[test]
fn codex_relay_normalizes_root_base_url_to_v1() {
    with_temp_home("codex-root-base-url", |_home| {
        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: vec![sample_provider("https://relay.example", "sk-test")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        };

        apply_relay_configs(&[agent]);

        let config_path = crate::paths::pad_codex_config_path();
        let config = std::fs::read_to_string(&config_path).expect("read codex config");
        let parsed: toml::Value = config.parse().expect("parse codex config");
        assert_eq!(
            parsed
                .get("model_provider")
                .and_then(|value| value.as_str()),
            Some("relay_a")
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|value| value.get("relay_a"))
                .and_then(|value| value.get("base_url"))
                .and_then(|value| value.as_str()),
            Some("https://relay.example/v1")
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|value| value.get("relay_a"))
                .and_then(|value| value.get("wire_api"))
                .and_then(|value| value.as_str()),
            None
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|value| value.get("relay_a"))
                .and_then(|value| value.get("requires_openai_auth"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );

        let auth_path = crate::paths::pad_codex_auth_path();
        let auth: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&auth_path).expect("read codex auth"))
                .expect("parse codex auth");
        assert_eq!(
            auth.get("OPENAI_API_KEY").and_then(|value| value.as_str()),
            Some("sk-test")
        );
        assert_eq!(
            auth.get("auth_mode").and_then(|value| value.as_str()),
            Some("apikey")
        );
    });
}

#[test]
fn codex_export_writes_pad_yaml_without_wire_api() {
    with_temp_home("codex-export", |home| {
        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: vec![sample_provider("https://relay.example", "sk-test")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        };

        let path = write_codex_relay_export(&agent).expect("write export");
        assert_eq!(path, home.join(".pad").join("relay.yaml"));

        let exported = std::fs::read_to_string(path).expect("read export");
        assert!(exported.contains("version: 1"));
        assert!(exported.contains("codex:"));
        assert!(exported.contains("active_provider: 0"));
        assert!(exported.contains("provider_name: \"relay_a\""));
        assert!(exported.contains("base_url: \"https://relay.example/v1\""));
        assert!(exported.contains("api_key: \"sk-test\""));
        assert!(!exported.contains("wire_api"));
    });
}

#[test]
fn codex_import_restores_exported_pad_yaml() {
    with_temp_home("codex-import", |_home| {
        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: vec![
                sample_provider("https://relay.example", "sk-test"),
                sample_provider("https://relay-b.example/v1", "sk-test-2"),
            ],
            active_provider: Some(1),
            default_model: String::new(),
            small_model: String::new(),
        };

        let _ = write_codex_relay_export(&agent).expect("write export");
        let (providers, active_provider, _path) = read_codex_relay_import().expect("read import");

        assert_eq!(providers.len(), 2);
        assert_eq!(active_provider, Some(1));
        assert_eq!(providers[0].label, "Relay A");
        assert_eq!(providers[0].base_url, "https://relay.example/v1");
        assert_eq!(providers[0].api_key, "sk-test");
        assert_eq!(providers[1].base_url, "https://relay-b.example/v1");
        assert_eq!(providers[1].api_key, "sk-test-2");
        assert!(providers
            .iter()
            .all(|provider| provider.wire_api.is_empty()));
    });
}

#[test]
fn codex_relay_preserves_explicit_v1_base_url() {
    with_temp_home("codex-v1-base-url", |_home| {
        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: vec![sample_provider("https://relay.example/v1", "sk-test")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        };

        apply_relay_configs(&[agent]);

        let config_path = crate::paths::pad_codex_config_path();
        let config = std::fs::read_to_string(&config_path).expect("read codex config");
        let parsed: toml::Value = config.parse().expect("parse codex config");
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|value| value.get("relay_a"))
                .and_then(|value| value.get("base_url"))
                .and_then(|value| value.as_str()),
            Some("https://relay.example/v1")
        );
    });
}
