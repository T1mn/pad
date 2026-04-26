#[test]
fn incomplete_codex_provider_restores_native_config() {
    let agent = AgentConfig {
        name: "codex".into(),
        cmd: "codex".into(),
        providers: vec![sample_provider("http://relay.example", "")],
        active_provider: Some(0),
        default_model: String::new(),
        small_model: String::new(),
        base_url: None,
        api_key: None,
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
        base_url: None,
        api_key: None,
    };

    assert!(!should_restore_native_codex_config(&agent));
}

#[test]
fn codex_relay_normalizes_root_base_url_to_v1() {
    with_temp_home("codex-root-base-url", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: vec![sample_provider("https://relay.example", "sk-test")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        apply_relay_configs(&[agent]);

        let config_path = codex_dir.join("config.toml");
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

        let auth_path = codex_dir.join("auth.json");
        let auth: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&auth_path).expect("read codex auth"))
                .expect("parse codex auth");
        assert_eq!(
            auth.get("OPENAI_API_KEY").and_then(|value| value.as_str()),
            Some("sk-test")
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
            base_url: None,
            api_key: None,
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
            base_url: None,
            api_key: None,
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
    with_temp_home("codex-v1-base-url", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: vec![sample_provider("https://relay.example/v1", "sk-test")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        apply_relay_configs(&[agent]);

        let config_path = codex_dir.join("config.toml");
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
            base_url: None,
            api_key: None,
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

#[test]
fn gemini_provider_writes_env_and_preserves_settings_json() {
    with_temp_home("gemini-write", |home| {
        let gemini_dir = home.join(".gemini");
        std::fs::create_dir_all(&gemini_dir).expect("create gemini dir");
        let settings_path = gemini_dir.join("settings.json");
        let env_path = gemini_dir.join(".env");
        std::fs::write(
            &settings_path,
            r#"{"mcpServers":{"echo":{"command":"echo"}},"apiUrl":"old","apiKey":"old"}"#,
        )
        .expect("seed gemini settings");
        std::fs::write(&env_path, "KEEP_ME=1\n").expect("seed gemini env");

        let agent = AgentConfig {
            name: "gemini".into(),
            cmd: "gemini".into(),
            providers: vec![sample_provider("https://gemini-relay.example", "gm-test")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };
        apply_relay_configs(&[agent]);

        let env = parse_env_file(&std::fs::read_to_string(&env_path).expect("read env"));
        assert_eq!(
            env.get("GOOGLE_GEMINI_BASE_URL").map(String::as_str),
            Some("https://gemini-relay.example")
        );
        assert_eq!(
            env.get("GEMINI_API_KEY").map(String::as_str),
            Some("gm-test")
        );
        assert_eq!(env.get("KEEP_ME").map(String::as_str), Some("1"));

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert_eq!(
            value
                .pointer("/security/auth/selectedType")
                .and_then(|v| v.as_str()),
            Some("apiKey")
        );
        assert!(value.get("mcpServers").is_some());
        assert!(value.get("apiUrl").is_none());
        assert!(value.get("apiKey").is_none());
    });
}

#[test]
fn incomplete_gemini_provider_restores_original_files() {
    with_temp_home("gemini-restore", |home| {
        let gemini_dir = home.join(".gemini");
        std::fs::create_dir_all(&gemini_dir).expect("create gemini dir");
        let settings_path = gemini_dir.join("settings.json");
        let env_path = gemini_dir.join(".env");
        std::fs::write(
            &settings_path,
            r#"{"mcpServers":{"echo":{"command":"echo"}}}"#,
        )
        .expect("seed settings");
        std::fs::write(&env_path, "KEEP_ME=1\n").expect("seed env");

        let complete = AgentConfig {
            name: "gemini".into(),
            cmd: "gemini".into(),
            providers: vec![sample_provider("https://gemini-relay.example", "gm-test")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };
        apply_relay_configs(&[complete]);

        let incomplete = AgentConfig {
            name: "gemini".into(),
            cmd: "gemini".into(),
            providers: vec![sample_provider("https://gemini-relay.example", "")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };
        apply_relay_configs(&[incomplete]);

        assert_eq!(
            std::fs::read_to_string(&env_path).expect("read restored env"),
            "KEEP_ME=1\n"
        );
        let restored: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert!(restored.pointer("/mcpServers/echo").is_some());
        assert!(restored.pointer("/security/auth").is_none());
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
            base_url: None,
            api_key: None,
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
            base_url: None,
            api_key: None,
        };

        apply_relay_configs(&[agent]);

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).expect("read"))
                .expect("parse");
        assert!(value.pointer("/provider/relay-a").is_none());
        assert!(value.get("model").is_none());
    });
}

