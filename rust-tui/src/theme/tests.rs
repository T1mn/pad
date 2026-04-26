use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock theme tests");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let home = std::env::temp_dir().join(format!("pad-theme-{name}-{stamp}"));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let result = f();

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);
    result
}

#[test]
fn config_round_trips_opencode_provider_models() {
    with_temp_home("opencode-roundtrip", || {
        let mut config = Config::default();
        config.agent_permissions.codex_auto_full_access = false;
        config.codex.fast_mode = true;
        config.codex.multi_agent = true;
        config.codex.web_search = "live".into();
        config.codex.status_line_model_with_reasoning = true;
        config.codex.status_line_context_remaining = true;
        config.codex.status_line_current_dir = false;
        config.codex.jailbreak_prompt_file = true;
        config.codex.index_prompt_file = true;
        config.codex.title_summary = true;
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
        config.save();

        let loaded = Config::load();
        assert!(!loaded.agent_permissions.codex_auto_full_access);
        assert!(loaded.agent_permissions.claude_auto_full_access);
        assert!(loaded.codex.fast_mode);
        assert!(loaded.codex.multi_agent);
        assert_eq!(loaded.codex.web_search, "live");
        assert!(loaded.codex.status_line_model_with_reasoning);
        assert!(loaded.codex.status_line_context_remaining);
        assert!(!loaded.codex.status_line_current_dir);
        assert!(loaded.codex.jailbreak_prompt_file);
        assert!(loaded.codex.index_prompt_file);
        assert!(loaded.codex.title_summary);
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

#[test]
fn config_save_omits_wire_api_entries() {
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
            models: Vec::new(),
            test_status: None,
            test_http_status: None,
            test_latency_ms: None,
            test_result: None,
        });
        codex.active_provider = Some(0);
        config.save();

        let saved = std::fs::read_to_string(Config::config_path()).expect("read saved config");
        assert!(!saved.contains("wire_api"));
    });
}

#[test]
fn config_loads_legacy_codex_prompt_file_as_jailbreak_prompt_file() {
    with_temp_home("legacy-codex-prompt-file", || {
        let config_path = crate::paths::config_path();
        std::fs::create_dir_all(config_path.parent().expect("config parent"))
            .expect("create config dir");
        std::fs::write(&config_path, "[codex]\nprompt_file = true\n").expect("write legacy config");

        let loaded = Config::load();

        assert!(loaded.codex.jailbreak_prompt_file);
    });
}

#[test]
fn config_defaults_agent_permissions_to_enabled() {
    with_temp_home("permissions-default", || {
        let config = Config::default();
        config.save();

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

#[test]
fn config_round_trips_sound_section() {
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
        config.save();

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

#[test]
fn config_normalizes_invalid_sound_presets() {
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

#[test]
fn codex_base_url_candidates_try_root_and_v1_variants() {
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

#[test]
fn codex_base_url_prefers_v1_for_root_inputs() {
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

#[test]
fn resolved_config_path_prefers_pad_home_over_legacy_path() {
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

#[test]
fn load_from_path_reports_invalid_toml() {
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

#[test]
fn readability_boost_keeps_status_text_close_to_primary_fg() {
    let theme = Theme::by_name("catppuccin");
    assert_eq!(theme.status_fg, theme.fg);
}

#[test]
fn readability_boost_lifts_comment_contrast() {
    let boosted = Theme::by_name("one-dark");
    assert_ne!(boosted.comment, Color::Rgb(92, 99, 112));
}
