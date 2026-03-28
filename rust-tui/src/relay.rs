use crate::theme::AgentConfig;
use std::path::PathBuf;

/// Apply the active provider's relay/proxy config to each agent's native config files
pub fn apply_relay_configs(agents: &[AgentConfig]) {
    for agent in agents {
        match agent.name.as_str() {
            "claude" => {
                apply_standard_agent_config(agent, ".claude", "settings.json", "apiUrl", "apiKey")
            }
            "codex" => apply_codex_agent_config(agent),
            "gemini-cli" | "gemini" => {
                apply_standard_agent_config(agent, ".gemini", "settings.json", "apiUrl", "apiKey")
            }
            _ => {}
        }
    }
}

fn apply_standard_agent_config(
    agent: &AgentConfig,
    dir_name: &str,
    file_name: &str,
    url_key: &str,
    api_key_key: &str,
) {
    let prov = match agent.active() {
        Some(p) => p,
        None => return,
    };
    if prov.base_url.is_empty() && prov.api_key.is_empty() {
        return;
    }

    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(dir_name)
        .join(file_name);

    let mut obj = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if !prov.base_url.is_empty() {
        obj[url_key] = serde_json::Value::String(prov.base_url.clone());
    }
    if !prov.api_key.is_empty() {
        obj[api_key_key] = serde_json::Value::String(prov.api_key.clone());
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(&obj).unwrap_or_default(),
    );
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
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("config.toml")
}

fn codex_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("codex-config.pre-pad.toml")
}

fn codex_auth_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("auth.json")
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
    let mut obj = serde_json::from_str::<serde_json::Value>(content)
        .unwrap_or_else(|_| serde_json::json!({}));
    if !obj.is_object() {
        obj = serde_json::json!({});
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

    fn sample_provider(base_url: &str, api_key: &str) -> ProviderConfig {
        ProviderConfig {
            label: "Relay A".into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            env_key: String::new(),
            wire_api: "responses".into(),
            test_status: None,
            test_http_status: None,
            test_latency_ms: None,
            test_result: None,
        }
    }

    #[test]
    fn incomplete_codex_provider_restores_native_config() {
        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: vec![sample_provider("http://relay.example", "")],
            active_provider: Some(0),
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
            base_url: None,
            api_key: None,
        };

        assert!(!should_restore_native_codex_config(&agent));
    }
}
