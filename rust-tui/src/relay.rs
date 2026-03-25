use crate::theme::AgentConfig;
use std::path::PathBuf;

/// Apply the active provider's relay/proxy config to each agent's native config files
pub fn apply_relay_configs(agents: &[AgentConfig]) {
    for agent in agents {
        let prov = match agent.active() {
            Some(p) => p,
            None => continue,
        };
        if prov.base_url.is_empty() && prov.api_key.is_empty() {
            continue;
        }
        let base_url = if prov.base_url.is_empty() {
            None
        } else {
            Some(prov.base_url.as_str())
        };
        let api_key = if prov.api_key.is_empty() {
            None
        } else {
            Some(prov.api_key.as_str())
        };
        match agent.name.as_str() {
            "claude" => apply_claude_config(base_url, api_key),
            "codex" => apply_codex_config(base_url, api_key),
            "gemini-cli" | "gemini" => apply_gemini_config(base_url, api_key),
            _ => {}
        }
    }
}

fn apply_claude_config(base_url: Option<&str>, api_key: Option<&str>) {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("settings.json");

    let mut obj = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(url) = base_url {
        obj["apiUrl"] = serde_json::Value::String(url.to_string());
    }
    if let Some(key) = api_key {
        obj["apiKey"] = serde_json::Value::String(key.to_string());
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(&obj).unwrap_or_default(),
    );
}

fn apply_codex_config(base_url: Option<&str>, api_key: Option<&str>) {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("config.toml");

    let mut content = if path.exists() {
        std::fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };

    if let Some(url) = base_url {
        content = set_toml_value(&content, "base_url", url);
    }
    if let Some(key) = api_key {
        content = set_toml_value(&content, "api_key", key);
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, content);
}

fn apply_gemini_config(base_url: Option<&str>, api_key: Option<&str>) {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".gemini")
        .join("settings.json");

    let mut obj = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(url) = base_url {
        obj["apiUrl"] = serde_json::Value::String(url.to_string());
    }
    if let Some(key) = api_key {
        obj["apiKey"] = serde_json::Value::String(key.to_string());
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(&obj).unwrap_or_default(),
    );
}

/// Simple helper to set a key=value in a TOML string
fn set_toml_value(content: &str, key: &str, value: &str) -> String {
    let prefix = format!("{} = ", key);
    let new_line = format!("{} = \"{}\"", key, value);

    let mut found = false;
    let mut lines: Vec<String> = content
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(&prefix) {
                found = true;
                new_line.clone()
            } else {
                line.to_string()
            }
        })
        .collect();

    if !found {
        lines.push(new_line);
    }

    lines.join("\n")
}
