use crate::theme::AgentConfig;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Apply the active provider's relay/proxy config to each agent's native config files
pub fn apply_relay_configs(agents: &[AgentConfig]) {
    for agent in agents {
        match agent.name.as_str() {
            "claude" => apply_claude_agent_config(agent),
            "codex" => apply_codex_agent_config(agent),
            "gemini-cli" | "gemini" => apply_gemini_agent_config(agent),
            "opencode" => apply_opencode_agent_config(agent),
            _ => {}
        }
    }
}

fn apply_claude_agent_config(agent: &AgentConfig) {
    let path = claude_settings_path();

    if should_restore_standard_relay_config(agent) {
        restore_file(&path, &claude_backup_path());
        return;
    }

    let Some(prov) = agent.active() else {
        restore_file(&path, &claude_backup_path());
        return;
    };

    let content = std::fs::read_to_string(&path).unwrap_or_default();
    if !has_backup(&claude_backup_path()) {
        let _ = backup_file(&claude_backup_path(), &content);
    }

    let updated = update_claude_settings_config(&content, &prov.base_url, &prov.api_key);
    write_text_file(&path, &updated);
}

fn apply_gemini_agent_config(agent: &AgentConfig) {
    let env_path = gemini_env_path();
    let settings_path = gemini_settings_path();

    if should_restore_standard_relay_config(agent) {
        restore_file(&env_path, &gemini_env_backup_path());
        restore_file(&settings_path, &gemini_settings_backup_path());
        return;
    }

    let Some(prov) = agent.active() else {
        restore_file(&env_path, &gemini_env_backup_path());
        restore_file(&settings_path, &gemini_settings_backup_path());
        return;
    };

    let env_content = std::fs::read_to_string(&env_path).unwrap_or_default();
    let settings_content = std::fs::read_to_string(&settings_path).unwrap_or_default();

    if !has_backup(&gemini_env_backup_path()) {
        let _ = backup_file(&gemini_env_backup_path(), &env_content);
    }
    if !has_backup(&gemini_settings_backup_path()) {
        let _ = backup_file(&gemini_settings_backup_path(), &settings_content);
    }

    let updated_env = update_gemini_env_config(&env_content, &prov.base_url, &prov.api_key);
    let updated_settings = update_gemini_settings_config(&settings_content);
    write_text_file(&env_path, &updated_env);
    write_text_file(&settings_path, &updated_settings);
}

fn should_restore_standard_relay_config(agent: &AgentConfig) -> bool {
    let Some(prov) = agent.active() else {
        return true;
    };
    prov.base_url.trim().is_empty() || prov.api_key.trim().is_empty()
}

fn opencode_config_path() -> PathBuf {
    if let Some(path) = std::env::var_os("OPENCODE_CONFIG") {
        return PathBuf::from(path);
    }

    if let Some(dir) = std::env::var_os("OPENCODE_CONFIG_DIR") {
        return PathBuf::from(dir).join("opencode.json");
    }

    home_dir()
        .join(".config")
        .join("opencode")
        .join("opencode.json")
}

fn opencode_managed_state_path() -> PathBuf {
    crate::paths::pad_home_dir().join("opencode-relay-state.json")
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn claude_settings_path() -> PathBuf {
    home_dir().join(".claude").join("settings.json")
}

fn claude_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("claude-settings.pre-pad.json")
}

fn gemini_settings_path() -> PathBuf {
    home_dir().join(".gemini").join("settings.json")
}

fn gemini_env_path() -> PathBuf {
    home_dir().join(".gemini").join(".env")
}

fn gemini_settings_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("gemini-settings.pre-pad.json")
}

fn gemini_env_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("gemini-env.pre-pad")
}

fn has_backup(path: &Path) -> bool {
    path.exists()
}

fn backup_file(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

fn restore_file(path: &Path, backup_path: &Path) {
    let Ok(content) = std::fs::read_to_string(backup_path) else {
        return;
    };
    write_text_file(path, &content);
}

fn write_text_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, content);
}

fn read_json_value(path: &Path, fallback: serde_json::Value) -> serde_json::Value {
    let Ok(content) = std::fs::read_to_string(path) else {
        return fallback;
    };
    let parsed = serde_json::from_str::<serde_json::Value>(&content).unwrap_or(fallback);
    if parsed.is_object() {
        parsed
    } else {
        json!({})
    }
}

fn write_json_value(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(mut content) = serde_json::to_string_pretty(value) else {
        return;
    };
    if !content.ends_with('\n') {
        content.push('\n');
    }
    let _ = std::fs::write(path, content);
}

fn read_opencode_managed_keys() -> BTreeSet<String> {
    let state_path = opencode_managed_state_path();
    let value = read_json_value(&state_path, json!({ "provider_keys": [] }));
    value
        .get("provider_keys")
        .and_then(|items| items.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect()
}

fn write_opencode_managed_keys(keys: &BTreeSet<String>) {
    let value = json!({
        "provider_keys": keys.iter().collect::<Vec<_>>()
    });
    write_json_value(&opencode_managed_state_path(), &value);
}

fn apply_opencode_agent_config(agent: &AgentConfig) {
    let path = opencode_config_path();
    let mut root = read_json_value(
        &path,
        json!({ "$schema": "https://opencode.ai/config.json" }),
    );

    if root.get("$schema").is_none() {
        root.as_object_mut()
            .expect("opencode config object")
            .insert(
                "$schema".to_string(),
                serde_json::Value::String("https://opencode.ai/config.json".to_string()),
            );
    }

    let previous_managed = read_opencode_managed_keys();
    let current_managed: BTreeSet<String> = agent
        .providers
        .iter()
        .filter_map(|provider| {
            let key = provider.opencode_provider_key().trim();
            if key.is_empty() {
                None
            } else {
                Some(key.to_string())
            }
        })
        .collect();

    let provider_entry = root
        .as_object_mut()
        .expect("opencode root object")
        .entry("provider".to_string())
        .or_insert_with(|| json!({}));
    if !provider_entry.is_object() {
        *provider_entry = json!({});
    }
    let provider_map = provider_entry
        .as_object_mut()
        .expect("opencode provider object");

    for removed_key in previous_managed.difference(&current_managed) {
        provider_map.remove(removed_key);
    }

    for provider in &agent.providers {
        let provider_key = provider.opencode_provider_key().trim();
        if provider_key.is_empty() {
            continue;
        }

        let models = provider
            .models
            .iter()
            .filter(|model| !model.id.trim().is_empty())
            .map(|model| {
                let display_name = if model.name.trim().is_empty() {
                    model.id.trim()
                } else {
                    model.name.trim()
                };
                (
                    model.id.trim().to_string(),
                    json!({
                        "name": display_name,
                    }),
                )
            })
            .collect::<serde_json::Map<String, serde_json::Value>>();

        let mut options = serde_json::Map::new();
        if !provider.base_url.trim().is_empty() {
            options.insert(
                "baseURL".to_string(),
                serde_json::Value::String(provider.base_url.trim().to_string()),
            );
        }
        if !provider.api_key.trim().is_empty() {
            options.insert(
                "apiKey".to_string(),
                serde_json::Value::String(provider.api_key.clone()),
            );
        }

        let config = json!({
            "npm": provider.opencode_npm_package(),
            "name": provider.label,
            "options": options,
            "models": models,
        });
        provider_map.insert(provider_key.to_string(), config);
    }

    let valid_models: BTreeSet<String> = agent
        .opencode_model_options()
        .into_iter()
        .map(|(value, _)| value)
        .collect();

    if !agent.default_model.trim().is_empty() && valid_models.contains(&agent.default_model) {
        root.as_object_mut().expect("opencode root object").insert(
            "model".to_string(),
            serde_json::Value::String(agent.default_model.clone()),
        );
    } else if root
        .get("model")
        .and_then(|value| value.as_str())
        .map(|value| {
            previous_managed
                .iter()
                .any(|key| value.starts_with(&format!("{key}/")))
        })
        .unwrap_or(false)
    {
        root.as_object_mut()
            .expect("opencode root object")
            .remove("model");
    }

    if !agent.small_model.trim().is_empty() && valid_models.contains(&agent.small_model) {
        root.as_object_mut().expect("opencode root object").insert(
            "small_model".to_string(),
            serde_json::Value::String(agent.small_model.clone()),
        );
    } else if root
        .get("small_model")
        .and_then(|value| value.as_str())
        .map(|value| {
            previous_managed
                .iter()
                .any(|key| value.starts_with(&format!("{key}/")))
        })
        .unwrap_or(false)
    {
        root.as_object_mut()
            .expect("opencode root object")
            .remove("small_model");
    }

    write_json_value(&path, &root);
    write_opencode_managed_keys(&current_managed);
}

fn update_claude_settings_config(content: &str, base_url: &str, api_key: &str) -> String {
    let mut obj = parse_json_object(content);
    obj.as_object_mut()
        .expect("claude settings root object")
        .remove("apiUrl");
    obj.as_object_mut()
        .expect("claude settings root object")
        .remove("apiKey");

    let env = obj
        .as_object_mut()
        .expect("claude settings root object")
        .entry("env".to_string())
        .or_insert_with(|| json!({}));
    if !env.is_object() {
        *env = json!({});
    }

    let env_obj = env.as_object_mut().expect("claude env object");
    env_obj.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        serde_json::Value::String(base_url.to_string()),
    );
    env_obj.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        serde_json::Value::String(api_key.to_string()),
    );

    serialize_json_pretty(&obj)
}

fn update_gemini_settings_config(content: &str) -> String {
    let mut obj = parse_json_object(content);
    obj.as_object_mut()
        .expect("gemini settings root object")
        .remove("apiUrl");
    obj.as_object_mut()
        .expect("gemini settings root object")
        .remove("apiKey");

    let security = obj
        .as_object_mut()
        .expect("gemini settings root object")
        .entry("security".to_string())
        .or_insert_with(|| json!({}));
    if !security.is_object() {
        *security = json!({});
    }

    let auth = security
        .as_object_mut()
        .expect("gemini security object")
        .entry("auth".to_string())
        .or_insert_with(|| json!({}));
    if !auth.is_object() {
        *auth = json!({});
    }

    auth.as_object_mut().expect("gemini auth object").insert(
        "selectedType".to_string(),
        serde_json::Value::String("apiKey".to_string()),
    );

    serialize_json_pretty(&obj)
}

fn update_gemini_env_config(content: &str, base_url: &str, api_key: &str) -> String {
    let mut env = parse_env_file(content);
    env.insert("GOOGLE_GEMINI_BASE_URL".to_string(), base_url.to_string());
    env.insert("GEMINI_API_KEY".to_string(), api_key.to_string());
    serialize_env_file(&env)
}

fn parse_json_object(content: &str) -> serde_json::Value {
    let mut obj = serde_json::from_str::<serde_json::Value>(content).unwrap_or_else(|_| json!({}));
    if !obj.is_object() {
        obj = json!({});
    }
    obj
}

fn serialize_json_pretty(value: &serde_json::Value) -> String {
    let mut serialized = serde_json::to_string_pretty(value).unwrap_or_default();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    serialized
}

fn parse_env_file(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

fn serialize_env_file(map: &BTreeMap<String, String>) -> String {
    if map.is_empty() {
        return String::new();
    }

    let mut serialized = map
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    serialized.push('\n');
    serialized
}

fn apply_codex_agent_config(agent: &AgentConfig) {
    let path = codex_config_path();
    let auth_path = codex_auth_path();

    if should_restore_native_codex_config(agent) {
        restore_codex_config();
        restore_codex_auth();
        return;
    }

    if let Some(prov) = agent.active() {
        let api_key = prov.codex_auth_token().unwrap_or_default();

        let content = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };
        let auth_content = if auth_path.exists() {
            std::fs::read_to_string(&auth_path).unwrap_or_default()
        } else {
            String::new()
        };

        if !has_pad_codex_backup() {
            let _ = backup_codex_config(&content);
        }
        if !has_pad_codex_auth_backup() {
            let _ = backup_codex_auth(&auth_content);
        }

        let updated = update_codex_provider_config(
            &content,
            &prov.codex_provider_name(),
            &prov.label,
            &prov.base_url,
            prov.codex_wire_api(),
        );
        let updated_auth = update_codex_auth_config(&auth_content, &api_key);

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Some(parent) = auth_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, updated);
        let _ = std::fs::write(&auth_path, updated_auth);
    } else {
        restore_codex_config();
        restore_codex_auth();
    }
}

fn should_restore_native_codex_config(agent: &AgentConfig) -> bool {
    let Some(prov) = agent.active() else {
        return true;
    };
    prov.base_url.trim().is_empty() || prov.codex_auth_token().is_none()
}

fn codex_config_path() -> PathBuf {
    home_dir().join(".codex").join("config.toml")
}

fn codex_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("codex-config.pre-pad.toml")
}

fn codex_auth_path() -> PathBuf {
    home_dir().join(".codex").join("auth.json")
}

fn codex_auth_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("codex-auth.pre-pad.json")
}

fn has_pad_codex_backup() -> bool {
    codex_backup_path().exists()
}

fn has_pad_codex_auth_backup() -> bool {
    codex_auth_backup_path().exists()
}

fn backup_codex_config(content: &str) -> std::io::Result<()> {
    let backup = codex_backup_path();
    if let Some(parent) = backup.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(backup, content)
}

fn backup_codex_auth(content: &str) -> std::io::Result<()> {
    let backup = codex_auth_backup_path();
    if let Some(parent) = backup.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(backup, content)
}

fn restore_codex_config() {
    let path = codex_config_path();
    let backup = codex_backup_path();
    let Ok(content) = std::fs::read_to_string(&backup) else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, content);
}

fn restore_codex_auth() {
    let path = codex_auth_path();
    let backup = codex_auth_backup_path();
    let Ok(content) = std::fs::read_to_string(&backup) else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, content);
}

fn update_codex_provider_config(
    content: &str,
    provider_name: &str,
    provider_label: &str,
    base_url: &str,
    wire_api: &str,
) -> String {
    let mut doc = content
        .parse::<toml::Value>()
        .unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()));

    let root = doc.as_table_mut().expect("root toml value must be a table");
    root.insert(
        "model_provider".to_string(),
        toml::Value::String(provider_name.to_string()),
    );

    let providers = root
        .entry("model_providers")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

    if !providers.is_table() {
        *providers = toml::Value::Table(toml::map::Map::new());
    }

    let providers_table = providers
        .as_table_mut()
        .expect("model_providers must be a table");
    let provider_entry = providers_table
        .entry(provider_name.to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

    if !provider_entry.is_table() {
        *provider_entry = toml::Value::Table(toml::map::Map::new());
    }

    let provider_table = provider_entry
        .as_table_mut()
        .expect("provider entry must be a table");
    provider_table.insert(
        "base_url".to_string(),
        toml::Value::String(base_url.to_string()),
    );
    provider_table.insert(
        "name".to_string(),
        toml::Value::String(provider_label.to_string()),
    );
    provider_table.insert(
        "wire_api".to_string(),
        toml::Value::String(wire_api.to_string()),
    );
    provider_table.insert(
        "requires_openai_auth".to_string(),
        toml::Value::Boolean(true),
    );
    provider_table.remove("env_key");

    let mut serialized = toml::to_string(&doc).unwrap_or_default();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    serialized
}

fn update_codex_auth_config(content: &str, api_key: &str) -> String {
    let mut obj = serde_json::from_str::<serde_json::Value>(content).unwrap_or_else(|_| json!({}));
    if !obj.is_object() {
        obj = json!({});
    }
    obj["OPENAI_API_KEY"] = serde_json::Value::String(api_key.to_string());
    let mut serialized = serde_json::to_string_pretty(&obj).unwrap_or_default();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    serialized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{AgentConfig, ProviderConfig};
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
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

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn temp_home(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("pad-relay-{name}-{stamp}"))
    }

    fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
        let _guard = test_lock().lock().expect("lock relay tests");
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
}
