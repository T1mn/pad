use super::codex::should_restore_native_codex_config;
use super::common::{
    claude_permission_state_path, codex_permission_state_path, opencode_managed_state_path,
    parse_env_file,
};
use super::{apply_relay_configs, apply_runtime_configs};
use crate::theme::{AgentConfig, AgentPermissionsConfig, CodexConfig, ProviderConfig};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_provider(base_url: &str, api_key: &str) -> ProviderConfig {
    ProviderConfig {
        label: "Relay A".into(),
        base_url: base_url.into(),
        api_key: api_key.into(),
        env_key: String::new(),
        wire_api: "responses".into(),
        provider_key: "relay-a".into(),
        npm_package: "@ai-sdk/openai-compatible".into(),
        models: Vec::new(),
        test_status: None,
        test_http_status: None,
        test_latency_ms: None,
        test_result: None,
    }
}

fn sample_permissions() -> AgentPermissionsConfig {
    AgentPermissionsConfig {
        codex_auto_full_access: true,
        claude_auto_full_access: true,
    }
}

fn sample_codex_config() -> CodexConfig {
    CodexConfig::default()
}

fn temp_home(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("pad-relay-{name}-{stamp}"))
}

fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock relay tests");
    let home = temp_home(name);
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let result = f(&home);

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);

    result
}

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
            Some("responses")
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

#[test]
fn runtime_configs_apply_codex_full_access_without_relay_provider() {
    with_temp_home("codex-permissions", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\napproval_policy = \"on-request\"\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        apply_runtime_configs(&[agent], &sample_permissions(), &sample_codex_config());

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("approval_policy = \"never\""));
        assert!(value.contains("sandbox_mode = \"danger-full-access\""));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_permission_fields_when_disabled() {
    with_temp_home("codex-permissions-restore", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\napproval_policy = \"on-request\"\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        apply_runtime_configs(
            std::slice::from_ref(&agent),
            &sample_permissions(),
            &sample_codex_config(),
        );

        let disabled = AgentPermissionsConfig {
            codex_auto_full_access: false,
            claude_auto_full_access: false,
        };
        apply_runtime_configs(&[agent], &disabled, &sample_codex_config());

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("approval_policy = \"on-request\""));
        assert!(!value.contains("sandbox_mode = \"danger-full-access\""));
        assert!(!codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_apply_codex_fast_mode_without_relay_provider() {
    with_temp_home("codex-fast-mode", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(&config_path, "model = \"gpt-5\"\n").expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.fast_mode = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("service_tier = \"fast\""));
        assert!(value.contains("fast_mode = true"));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_fast_fields_when_disabled() {
    with_temp_home("codex-fast-mode-restore", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\nservice_tier = \"default\"\n[features]\nfast_mode = false\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.fast_mode = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.fast_mode = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("service_tier = \"default\""));
        assert!(value.contains("fast_mode = false"));
    });
}

#[test]
fn runtime_configs_apply_codex_multi_agent_without_relay_provider() {
    with_temp_home("codex-multi-agent", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(&config_path, "model = \"gpt-5\"\n").expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.multi_agent = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("multi_agent = true"));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_multi_agent_when_disabled() {
    with_temp_home("codex-multi-agent-restore", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\n[features]\nmulti_agent = false\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.multi_agent = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.multi_agent = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("multi_agent = false"));
    });
}

#[test]
fn runtime_configs_apply_codex_web_search_without_relay_provider() {
    with_temp_home("codex-web-search", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(&config_path, "model = \"gpt-5\"\n").expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.web_search = "live".into();
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("web_search = \"live\""));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_web_search_when_defaulted() {
    with_temp_home("codex-web-search-restore", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(&config_path, "model = \"gpt-5\"\nweb_search = \"cached\"\n")
            .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.web_search = "live".into();
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.web_search = "default".into();
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("web_search = \"cached\""));
    });
}

#[test]
fn runtime_configs_apply_combined_codex_overlays_together() {
    with_temp_home("codex-combined-overlays", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\napproval_policy = \"on-request\"\nservice_tier = \"default\"\nweb_search = \"cached\"\n[features]\nfast_mode = false\nmulti_agent = false\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.fast_mode = true;
        codex.multi_agent = true;
        codex.web_search = "live".into();
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("approval_policy = \"never\""));
        assert!(value.contains("sandbox_mode = \"danger-full-access\""));
        assert!(value.contains("service_tier = \"fast\""));
        assert!(value.contains("web_search = \"live\""));
        assert!(value.contains("fast_mode = true"));
        assert!(value.contains("multi_agent = true"));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_combined_codex_overlays_to_original_values() {
    with_temp_home("codex-combined-restore", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\napproval_policy = \"on-request\"\nservice_tier = \"default\"\nweb_search = \"cached\"\n[features]\nfast_mode = false\nmulti_agent = false\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.fast_mode = true;
        codex.multi_agent = true;
        codex.web_search = "live".into();
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        let disabled = AgentPermissionsConfig {
            codex_auto_full_access: false,
            claude_auto_full_access: true,
        };
        codex.fast_mode = false;
        codex.multi_agent = false;
        codex.web_search = "default".into();
        apply_runtime_configs(&[agent], &disabled, &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("approval_policy = \"on-request\""));
        assert!(!value.contains("sandbox_mode = \"danger-full-access\""));
        assert!(value.contains("service_tier = \"default\""));
        assert!(value.contains("web_search = \"cached\""));
        assert!(value.contains("fast_mode = false"));
        assert!(value.contains("multi_agent = false"));
        assert!(!codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_apply_claude_full_access_without_relay_provider() {
    with_temp_home("claude-permissions", |home| {
        let claude_dir = home.join(".claude");
        std::fs::create_dir_all(&claude_dir).expect("create claude dir");
        let settings_path = claude_dir.join("settings.json");
        std::fs::write(
            &settings_path,
            r#"{"permissions":{"defaultMode":"ask"},"sandbox":{"enabled":true},"mcpServers":{"echo":{"command":"echo"}}}"#,
        )
        .expect("seed claude settings");

        let agent = AgentConfig {
            name: "claude".into(),
            cmd: "claude".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        apply_runtime_configs(&[agent], &sample_permissions(), &sample_codex_config());

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert_eq!(
            value
                .pointer("/permissions/defaultMode")
                .and_then(|v| v.as_str()),
            Some("bypassPermissions")
        );
        assert_eq!(
            value.pointer("/sandbox/enabled").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert!(value.pointer("/mcpServers/echo").is_some());
        assert!(claude_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_claude_permission_fields_when_disabled() {
    with_temp_home("claude-permissions-restore", |home| {
        let claude_dir = home.join(".claude");
        std::fs::create_dir_all(&claude_dir).expect("create claude dir");
        let settings_path = claude_dir.join("settings.json");
        std::fs::write(
            &settings_path,
            r#"{"permissions":{"defaultMode":"ask"},"sandbox":{"enabled":true},"env":{"KEEP":"1"}}"#,
        )
        .expect("seed claude settings");

        let agent = AgentConfig {
            name: "claude".into(),
            cmd: "claude".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        apply_runtime_configs(
            std::slice::from_ref(&agent),
            &sample_permissions(),
            &sample_codex_config(),
        );

        let disabled = AgentPermissionsConfig {
            codex_auto_full_access: false,
            claude_auto_full_access: false,
        };
        apply_runtime_configs(&[agent], &disabled, &sample_codex_config());

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert_eq!(
            value
                .pointer("/permissions/defaultMode")
                .and_then(|v| v.as_str()),
            Some("ask")
        );
        assert_eq!(
            value.pointer("/sandbox/enabled").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            value.pointer("/env/KEEP").and_then(|v| v.as_str()),
            Some("1")
        );
        assert!(!claude_permission_state_path().exists());
    });
}
