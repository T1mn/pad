mod claude {
    use super::*;
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
            assert!(value.pointer("/env/ANTHROPIC_API_KEY").is_none());
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
            assert!(value
                .pointer("/env/CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS")
                .is_none());
            assert!(value.pointer("/env/MAX_THINKING_TOKENS").is_none());
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
            false,
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
            false,
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

    #[test]
    fn claude_provider_clears_stale_default_model_env_when_unconfigured() {
        let updated = crate::relay::claude::update_claude_settings_config(
            r#"{"env":{"ANTHROPIC_API_KEY":"old","ANTHROPIC_MODEL":"old","ANTHROPIC_CUSTOM_MODEL_OPTION":"old","MAX_THINKING_TOKENS":"0"}}"#,
            "https://claude-relay.example",
            "sk-ant-test",
            "",
            false,
        );
        let value: serde_json::Value = serde_json::from_str(&updated).expect("parse");

        assert!(value.pointer("/env/ANTHROPIC_API_KEY").is_none());
        assert!(value.pointer("/env/ANTHROPIC_MODEL").is_none());
        assert!(value
            .pointer("/env/ANTHROPIC_CUSTOM_MODEL_OPTION")
            .is_none());
        assert!(value.pointer("/env/MAX_THINKING_TOKENS").is_none());
    }

    #[test]
    fn claude_provider_writes_disable_thinking_env_only_when_enabled() {
        let updated = crate::relay::claude::update_claude_settings_config(
            "{}",
            "https://claude-relay.example",
            "sk-ant-test",
            "",
            true,
        );
        let value: serde_json::Value = serde_json::from_str(&updated).expect("parse");

        assert_eq!(
            value
                .pointer("/env/CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS")
                .and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            value.pointer("/env/MAX_THINKING_TOKENS").and_then(|v| v.as_str()),
            Some("0")
        );
    }
}

mod claude_safety {
    use super::*;
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
}

mod codex {
    use super::*;
    include!("provider_configs/codex.rs");
}

mod deepseek {
    use super::*;
    #[cfg(unix)]
    fn deepseek_agent(secret: &str) -> AgentConfig {
        AgentConfig {
            name: "deepseek".into(),
            cmd: "claude".into(),
            providers: vec![sample_provider("https://relay.example", secret)],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn deepseek_launcher_keeps_secret_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        with_temp_home("deepseek-launcher-permissions", |home| {
            let secret = "sk-deepseek-private-test";
            apply_relay_configs(&[deepseek_agent(secret)]);

            let launcher = home.join(".pad").join("deepseek-cc");
            let content = std::fs::read_to_string(&launcher).expect("read DeepSeek launcher");
            let mode = std::fs::metadata(&launcher)
                .expect("stat DeepSeek launcher")
                .permissions()
                .mode()
                & 0o777;

            assert!(content.contains(secret), "launcher must contain the test secret");
            assert_eq!(mode, 0o700, "launcher must be owner read/write/execute");
        });
    }

    #[cfg(unix)]
    #[test]
    fn deepseek_launcher_atomically_replaces_existing_broad_file() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        with_temp_home("deepseek-launcher-replace", |home| {
            let pad_dir = home.join(".pad");
            let launcher = pad_dir.join("deepseek-cc");
            std::fs::create_dir_all(&pad_dir).expect("create pad dir");
            std::fs::write(&launcher, "old secret").expect("seed launcher");
            std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755))
                .expect("seed broad launcher mode");
            let old_inode = std::fs::metadata(&launcher).expect("stat old launcher").ino();

            apply_relay_configs(&[deepseek_agent("new secret")]);

            let metadata = std::fs::metadata(&launcher).expect("stat new launcher");
            let content = std::fs::read_to_string(&launcher).expect("read new launcher");
            assert!(content.contains("new secret"));
            assert!(!content.contains("old secret"));
            assert_ne!(metadata.ino(), old_inode, "launcher must be replaced by rename");
            assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
        });
    }

    #[cfg(unix)]
    #[test]
    fn deepseek_launcher_failure_preserves_existing_file() {
        use std::os::unix::fs::PermissionsExt;

        with_temp_home("deepseek-launcher-failure", |home| {
            let pad_dir = home.join(".pad");
            let launcher = pad_dir.join("deepseek-cc");
            std::fs::create_dir_all(&pad_dir).expect("create pad dir");
            std::fs::write(&launcher, "old secret").expect("seed launcher");
            std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o700))
                .expect("seed launcher mode");
            std::fs::set_permissions(&pad_dir, std::fs::Permissions::from_mode(0o555))
                .expect("make launcher dir read-only");

            apply_relay_configs(&[deepseek_agent("new secret")]);

            std::fs::set_permissions(&pad_dir, std::fs::Permissions::from_mode(0o755))
                .expect("restore launcher dir permissions");
            assert_eq!(
                std::fs::read_to_string(&launcher).expect("read preserved launcher"),
                "old secret"
            );
            let temp_files = std::fs::read_dir(&pad_dir)
                .expect("read launcher dir")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(".pad-tmp"))
                .map(|entry| entry.path())
                .collect::<Vec<_>>();
            assert!(temp_files.is_empty(), "leftover temp files: {temp_files:?}");
        });
    }
}

mod gemini {
    use super::*;
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
            };
            apply_relay_configs(&[complete]);

            let incomplete = AgentConfig {
                name: "gemini".into(),
                cmd: "gemini".into(),
                providers: vec![sample_provider("https://gemini-relay.example", "")],
                active_provider: Some(0),
                default_model: String::new(),
                small_model: String::new(),
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
}

mod opencode {
    use super::*;
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
}

mod opencode_runtime {
    use super::*;
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
}

mod opencode_safety {
    use super::*;
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
}

mod security {
    use super::*;
    #[cfg(unix)]
    fn assert_private(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        let mode = std::fs::metadata(path)
            .expect("stat provider file")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "provider file must be owner-only: {path:?}");
    }

    #[cfg(unix)]
    #[test]
    fn relay_provider_files_are_private() {
        with_temp_home("private-provider-files", |home| {
            let agents = [
                AgentConfig {
                    name: "claude".into(),
                    cmd: "claude".into(),
                    providers: vec![sample_provider("https://claude.example", "claude-secret")],
                    active_provider: Some(0),
                    default_model: String::new(),
                    small_model: String::new(),
                },
                AgentConfig {
                    name: "codex".into(),
                    cmd: "codex".into(),
                    providers: vec![sample_provider("https://codex.example", "codex-secret")],
                    active_provider: Some(0),
                    default_model: String::new(),
                    small_model: String::new(),
                },
            ];
            apply_relay_configs(&agents);

            for path in [
                home.join(".claude/settings.json"),
                crate::paths::pad_codex_config_path(),
                crate::paths::pad_codex_auth_path(),
                crate::paths::pad_home_dir().join("claude-settings.pre-pad.json"),
                crate::paths::pad_home_dir().join("codex-config.pre-pad.toml"),
                crate::paths::pad_home_dir().join("codex-auth.pre-pad.json"),
            ] {
                assert_private(&path);
            }
        });
    }
}
