pub(crate) mod config {
    use super::super::*;
    use super::support::with_temp_home;

    pub(crate) fn config_round_trips_opencode_provider_models() {
        with_temp_home("opencode-roundtrip", || {
            let mut config = Config::default();
            config.profile.name = "desktop-default".into();
            config
                .profile
                .set_permission_mode(ProfileConfig::WORKSPACE_FULL_ACCESS);
            config.profile.unattended = true;
            config.agent_permissions.codex_auto_full_access = false;
            config.codex.fast_mode = true;
            config.codex.goals = true;
            config.codex.multi_agent = true;
            config.codex.web_search = "live".into();
            config.codex.status_line_model_with_reasoning = true;
            config.codex.status_line_fast_mode = true;
            config.codex.status_line_five_hour_limit = true;
            config.codex.status_line_weekly_limit = true;
            config.codex.status_line_context_remaining = true;
            config.codex.status_line_current_dir = false;
            config.codex.jailbreak_prompt_file = true;
            config.codex.index_prompt_file = true;
            config.codex.title_summary = true;
            config.codex.show_qa_preview = true;
            config.display.agent_panel_width = Some(72);
            let opencode = config
                .agents
                .iter_mut()
                .find(|agent| agent.name == "opencode")
                .expect("opencode agent");
            opencode.providers.push(ProviderConfig {
                label: "Relay".into(),
                base_url: "https://relay.example/v1".into(),
                api_key: "sk-test".into(),
                env_key: String::new(),
                wire_api: "responses".into(),
                provider_key: "relay".into(),
                npm_package: "@ai-sdk/openai-compatible".into(),
                disable_thinking: false,
                models: vec![OpenCodeModelConfig {
                    id: "gpt-4o".into(),
                    name: "GPT-4o".into(),
                }],
                test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            });
            opencode.active_provider = Some(0);
            opencode.default_model = "relay/gpt-4o".into();
            opencode.small_model = "relay/gpt-4o-mini".into();
            config.save().expect("save config");

            let loaded = Config::load();
            assert_eq!(loaded.profile.name, "desktop-default");
            assert_eq!(
                loaded.profile.default_permission_mode,
                ProfileConfig::WORKSPACE_FULL_ACCESS
            );
            assert!(!loaded.profile.full_access);
            assert!(loaded.profile.unattended);
            assert!(!loaded.agent_permissions.codex_auto_full_access);
            assert!(loaded.agent_permissions.claude_auto_full_access);
            assert!(loaded.codex.fast_mode);
            assert!(loaded.codex.goals);
            assert!(loaded.codex.multi_agent);
            assert_eq!(loaded.codex.web_search, "live");
            assert!(loaded.codex.status_line_model_with_reasoning);
            assert!(loaded.codex.status_line_fast_mode);
            assert!(loaded.codex.status_line_five_hour_limit);
            assert!(loaded.codex.status_line_weekly_limit);
            assert!(loaded.codex.status_line_context_remaining);
            assert!(!loaded.codex.status_line_current_dir);
            assert!(loaded.codex.jailbreak_prompt_file);
            assert!(loaded.codex.index_prompt_file);
            assert!(loaded.codex.title_summary);
            assert!(loaded.codex.show_qa_preview);
            assert_eq!(loaded.display.agent_panel_width, Some(72));
            let opencode = loaded
                .agents
                .iter()
                .find(|agent| agent.name == "opencode")
                .expect("loaded opencode");
            assert_eq!(opencode.default_model, "relay/gpt-4o");
            assert_eq!(opencode.small_model, "");
            assert_eq!(opencode.providers.len(), 1);
            assert_eq!(opencode.providers[0].provider_key, "relay");
            assert_eq!(
                opencode.providers[0].npm_package,
                "@ai-sdk/openai-compatible"
            );
            assert_eq!(opencode.providers[0].models.len(), 1);
            assert_eq!(opencode.providers[0].models[0].id, "gpt-4o");
            assert_eq!(opencode.providers[0].models[0].name, "GPT-4o");
        });
    }

    pub(crate) fn config_save_omits_wire_api_entries() {
        with_temp_home("save-omits-wire-api", || {
            let mut config = Config::default();
            let codex = config
                .agents
                .iter_mut()
                .find(|agent| agent.name == "codex")
                .expect("codex agent");
            codex.providers.push(ProviderConfig {
                label: "Relay".into(),
                base_url: "https://relay.example/v1".into(),
                api_key: "sk-test".into(),
                env_key: String::new(),
                wire_api: "responses_websocket".into(),
                provider_key: "relay".into(),
                npm_package: "@ai-sdk/openai-compatible".into(),
                disable_thinking: false,
                models: Vec::new(),
                test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            });
            codex.active_provider = Some(0);
            config.save().expect("save config");

            let saved = std::fs::read_to_string(Config::config_path()).expect("read saved config");
            assert!(!saved.contains("wire_api"));
        });
    }

    pub(crate) fn config_loads_legacy_codex_prompt_file_as_jailbreak_prompt_file() {
        with_temp_home("legacy-codex-prompt-file", || {
            let config_path = crate::paths::config_path();
            std::fs::create_dir_all(config_path.parent().expect("config parent"))
                .expect("create config dir");
            std::fs::write(&config_path, "[codex]\nprompt_file = true\n")
                .expect("write legacy config");

            let loaded = Config::load();

            assert!(loaded.codex.jailbreak_prompt_file);
        });
    }

    pub(crate) fn config_defaults_agent_permissions_to_enabled() {
        with_temp_home("permissions-default", || {
            let config = Config::default();
            config.save().expect("save config");

            let loaded = Config::load();
            assert!(loaded.agent_permissions.codex_auto_full_access);
            assert!(loaded.agent_permissions.claude_auto_full_access);
            assert!(loaded.sound.enabled);
            assert!(loaded.sound.completion.enabled);
            assert_eq!(loaded.sound.completion.preset, "glass");
            assert!(!loaded.sound.approval.enabled);
            assert_eq!(loaded.sound.approval.preset, "ping");
        });
    }

    pub(crate) fn config_loads_profile_full_access_compatibility_alias() {
        with_temp_home("profile-full-access-alias", || {
            let path = Config::config_path();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create pad parent");
            }
            std::fs::write(
                &path,
                r#"[profile]
full_access = true
unattended = true

[agent_permissions]
codex_auto_full_access = false
claude_auto_full_access = false
"#,
            )
            .expect("write profile config");

            let loaded = Config::load();
            assert!(loaded.profile.full_access);
            assert_eq!(
                loaded.profile.default_permission_mode,
                ProfileConfig::SYSTEM_FULL_ACCESS
            );
            assert!(loaded.profile.unattended);
            // The new profile switch must not change the legacy provider flags.
            assert!(!loaded.agent_permissions.codex_auto_full_access);
            assert!(!loaded.agent_permissions.claude_auto_full_access);
        });
    }

    pub(crate) fn config_profile_mode_wins_over_compatibility_alias() {
        with_temp_home("profile-mode-precedence", || {
            let path = Config::config_path();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create pad parent");
            }
            std::fs::write(
                &path,
                r#"[profile]
default_permission_mode = "guarded"
full_access = true
"#,
            )
            .expect("write profile config");

            let loaded = Config::load();
            assert_eq!(
                loaded.profile.default_permission_mode,
                ProfileConfig::GUARDED
            );
            assert!(!loaded.profile.full_access);
        });
    }

    pub(crate) fn profile_config_normalizes_modes_and_keeps_alias_in_sync() {
        let mut profile = ProfileConfig::default();
        assert_eq!(
            ProfileConfig::normalized_permission_mode("workspace-full-access"),
            ProfileConfig::WORKSPACE_FULL_ACCESS
        );
        assert_eq!(
            ProfileConfig::normalized_permission_mode("unexpected"),
            ProfileConfig::GUARDED
        );

        profile.set_permission_mode(ProfileConfig::SYSTEM_FULL_ACCESS);
        assert!(profile.full_access);
        assert_eq!(
            profile.effective_permission_mode(),
            ProfileConfig::SYSTEM_FULL_ACCESS
        );
        profile.set_full_access(false);
        assert!(!profile.full_access);
        assert_eq!(profile.default_permission_mode, ProfileConfig::GUARDED);
    }

    pub(crate) fn resolved_config_path_prefers_pad_home_over_legacy_path() {
        with_temp_home("resolved-config-path", || {
            let pad_path = Config::config_path();
            let legacy_path = crate::paths::legacy_config_path();
            if let Some(parent) = legacy_path.parent() {
                std::fs::create_dir_all(parent).expect("create legacy parent");
            }
            std::fs::write(&legacy_path, "theme = \"legacy\"\n").expect("write legacy config");
            assert_eq!(Config::resolved_config_path(), Some(legacy_path.clone()));

            if let Some(parent) = pad_path.parent() {
                std::fs::create_dir_all(parent).expect("create pad parent");
            }
            std::fs::write(&pad_path, "theme = \"primary\"\n").expect("write primary config");
            assert_eq!(Config::resolved_config_path(), Some(pad_path));
        });
    }

    pub(crate) fn load_from_path_reports_invalid_toml() {
        with_temp_home("invalid-load-path", || {
            let path = Config::config_path();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create pad parent");
            }
            std::fs::write(&path, "not valid = [").expect("write invalid config");

            let err = Config::load_from_path(&path).expect_err("invalid TOML should fail");
            assert!(err.contains("parse"));
        });
    }
}
pub(crate) mod palette {
    use super::super::*;
    use ratatui::style::Color;

    pub(crate) fn readability_boost_keeps_status_text_close_to_primary_fg() {
        let theme = Theme::by_name("catppuccin");
        assert_eq!(theme.status_fg, theme.fg);
    }

    pub(crate) fn readability_boost_lifts_comment_contrast() {
        let boosted = Theme::by_name("one-dark");
        assert_ne!(boosted.comment, Color::Rgb(92, 99, 112));
    }
}
pub(crate) mod persist {
    use super::super::*;
    use super::support::with_temp_home;

    fn save_and_reload(api_key: &str, bot_token: &str) -> Config {
        let mut config = Config::default();
        config.telegram.bot_token = bot_token.to_string();
        let claude = config
            .agents
            .iter_mut()
            .find(|agent| agent.name == "claude")
            .expect("claude agent");
        claude.providers.push(ProviderConfig {
            label: "Relay".into(),
            base_url: "https://relay.example/v1".into(),
            api_key: api_key.into(),
            env_key: String::new(),
            wire_api: "responses".into(),
            provider_key: "relay".into(),
            npm_package: String::new(),
            disable_thinking: false,
            models: Vec::new(),
            test_status: None,
            test_http_status: None,
            test_latency_ms: None,
            test_result: None,
        });
        claude.active_provider = Some(0);
        config.save().expect("save config");
        Config::load()
    }

    fn loaded_api_key(config: &Config) -> String {
        config
            .agents
            .iter()
            .find(|agent| agent.name == "claude")
            .and_then(|agent| agent.providers.first())
            .map(|provider| provider.api_key.clone())
            .expect("claude provider survived the round trip")
    }

    pub(crate) fn api_key_with_backslashes_survives_round_trip() {
        with_temp_home("backslash-api-key", || {
            let loaded = save_and_reload(r"C:\path\to\key", r"123:AA\bb");
            assert_eq!(loaded_api_key(&loaded), r"C:\path\to\key");
            assert_eq!(loaded.telegram.bot_token, r"123:AA\bb");
        });
    }

    pub(crate) fn value_ending_with_backslash_does_not_break_the_file() {
        with_temp_home("trailing-backslash", || {
            let loaded = save_and_reload(r"secret\", "plain");
            assert_eq!(loaded_api_key(&loaded), r"secret\");
            // 结构没被吃掉：后面的 agent 仍然都在。
            assert_eq!(loaded.agents.len(), Config::default().agents.len());
        });
    }

    pub(crate) fn multiline_and_control_character_values_survive_round_trip() {
        with_temp_home("multiline-value", || {
            let loaded = save_and_reload("line1\nline2\n", "tab\tand\r\ncrlf");
            assert_eq!(loaded_api_key(&loaded), "line1\nline2\n");
            assert_eq!(loaded.telegram.bot_token, "tab\tand\r\ncrlf");
        });
    }

    pub(crate) fn broken_config_is_backed_up_before_falling_back_to_defaults() {
        with_temp_home("broken-config-backup", || {
            let path = Config::config_path();
            std::fs::create_dir_all(path.parent().expect("config parent"))
                .expect("create config dir");
            let broken = "[[agents]]\nname = \"claude\"\napi_key = \"C:\\path\"\n";
            std::fs::write(&path, broken).expect("write broken config");

            let report = Config::load_reported();
            let recovery = report.recovery.expect("parse failure must be reported");
            let backup = recovery.backup.expect("broken config must be backed up");

            assert_eq!(
                std::fs::read_to_string(&backup).expect("read backup"),
                broken,
                "original bytes must be recoverable after the default fallback"
            );
            assert!(recovery.error.contains("parse"));

            // 回退默认值后再保存一次（模拟用户随手改个设置），备份仍然在。
            report.config.save().expect("save defaults");
            assert!(backup.exists());
            assert_eq!(
                std::fs::read_to_string(&backup).expect("read backup again"),
                broken
            );
        });
    }

    #[cfg(unix)]
    pub(crate) fn saved_config_is_owner_only_readable() {
        use std::os::unix::fs::PermissionsExt;

        with_temp_home("config-permissions", || {
            let path = Config::config_path();
            std::fs::create_dir_all(path.parent().expect("config parent"))
                .expect("create config dir");
            std::fs::write(&path, "theme = \"default\"\n").expect("seed config");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
                .expect("loosen perms");

            Config::default().save().expect("save config");

            let mode = std::fs::metadata(&path)
                .expect("stat config")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600, "config.toml stores plaintext secrets");
        });
    }
}
pub(crate) mod provider {
    use super::super::*;

    pub(crate) fn codex_base_url_candidates_try_root_and_v1_variants() {
        assert_eq!(
            codex_api_base_candidates("https://relay.example"),
            vec![
                "https://relay.example".to_string(),
                "https://relay.example/v1".to_string()
            ]
        );
        assert_eq!(
            codex_api_base_candidates("https://relay.example/v1"),
            vec![
                "https://relay.example/v1".to_string(),
                "https://relay.example".to_string()
            ]
        );
        assert_eq!(
            codex_api_base_candidates("https://relay.example/openai/v1"),
            vec!["https://relay.example/openai/v1".to_string()]
        );
    }

    pub(crate) fn codex_base_url_prefers_v1_for_root_inputs() {
        assert_eq!(
            provider::codex_preferred_api_base_url("https://relay.example"),
            "https://relay.example/v1"
        );
        assert_eq!(
            provider::codex_preferred_api_base_url("https://relay.example/"),
            "https://relay.example/v1"
        );
        assert_eq!(
            provider::codex_preferred_api_base_url("https://relay.example/v1"),
            "https://relay.example/v1"
        );
        assert_eq!(
            provider::codex_preferred_api_base_url("https://relay.example/openai/v1"),
            "https://relay.example/openai/v1"
        );
    }
}
pub(crate) mod sound {
    use super::super::*;
    use super::support::with_temp_home;

    pub(crate) fn config_round_trips_sound_section() {
        with_temp_home("sound-roundtrip", || {
            let mut config = Config::default();
            config.sound.enabled = true;
            config.sound.completion.enabled = false;
            config.sound.completion.preset = "pop".into();
            config.sound.approval.enabled = true;
            config.sound.approval.preset = "glass".into();
            config.sound.timeout.enabled = true;
            config.sound.timeout.preset = "warning".into();
            config.sound.failure.enabled = true;
            config.sound.failure.preset = "alert".into();
            config.save().expect("save config");

            let loaded = Config::load();
            assert!(loaded.sound.enabled);
            assert!(!loaded.sound.completion.enabled);
            assert_eq!(loaded.sound.completion.preset, "pop");
            assert!(loaded.sound.approval.enabled);
            assert_eq!(loaded.sound.approval.preset, "glass");
            assert!(loaded.sound.timeout.enabled);
            assert_eq!(loaded.sound.timeout.preset, "warning");
            assert!(loaded.sound.failure.enabled);
            assert_eq!(loaded.sound.failure.preset, "alert");
        });
    }

    pub(crate) fn config_normalizes_invalid_sound_presets() {
        with_temp_home("sound-preset-normalize", || {
            let path = Config::config_path();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create pad parent");
            }
            std::fs::write(
                &path,
                r#"[sound]
    enabled = true

    [sound.completion]
    enabled = true
    preset = "bogus"

    [sound.approval]
    enabled = true
    preset = "also-bogus"
    "#,
            )
            .expect("write config");

            let loaded = Config::load();
            assert_eq!(loaded.sound.completion.preset, "glass");
            assert_eq!(loaded.sound.approval.preset, "ping");
            assert_eq!(loaded.sound.timeout.preset, "warning");
            assert_eq!(loaded.sound.failure.preset, "alert");
        });
    }
}
mod support {
    pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
        crate::test_support::with_temp_home("pad-theme", name, |_| f())
    }
}
