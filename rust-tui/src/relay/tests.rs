use super::codex::should_restore_native_codex_config;
use super::common::{
    claude_permission_state_path, codex_permission_state_path, opencode_managed_state_path,
    parse_env_file,
};
use super::{
    apply_relay_configs, apply_runtime_configs, read_codex_relay_import, write_codex_relay_export,
};
use crate::paths::{
    codex_index_prompt_file_path, codex_jailbreak_prompt_file_path,
    codex_selected_prompt_file_path, DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE,
    DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE,
};
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
    CodexConfig {
        goals: false,
        ..CodexConfig::default()
    }
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

mod provider_configs {
    use super::*;
    include!("tests/provider_configs.rs");
}

mod runtime_overlays {
    use super::*;
    include!("tests/runtime_overlays.rs");
}
