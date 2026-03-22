use crate::theme::AgentConfig;
use std::path::PathBuf;

/// Apply relay/proxy configurations to each agent's native config files
pub fn apply_relay_configs(agents: &[AgentConfig]) {
    for agent in agents {
        if agent.base_url.is_none() && agent.api_key.is_none() {
            continue;
        }
        match agent.name.as_str() {
            "claude" => apply_claude_config(agent),
            "codex" => apply_codex_config(agent),
            "gemini-cli" | "gemini" => apply_gemini_config(agent),
            _ => {}
        }
    }
}

fn apply_claude_config(agent: &AgentConfig) {
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

    if let Some(ref url) = agent.base_url {
        obj["apiUrl"] = serde_json::Value::String(url.clone());
    }
    if let Some(ref key) = agent.api_key {
        obj["apiKey"] = serde_json::Value::String(key.clone());
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&obj).unwrap_or_default());
}

fn apply_codex_config(agent: &AgentConfig) {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("config.toml");

    let mut content = if path.exists() {
        std::fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };

    if let Some(ref url) = agent.base_url {
        content = set_toml_value(&content, "base_url", url);
    }
    if let Some(ref key) = agent.api_key {
        content = set_toml_value(&content, "api_key", key);
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, content);
}

fn apply_gemini_config(agent: &AgentConfig) {
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

    if let Some(ref url) = agent.base_url {
        obj["apiUrl"] = serde_json::Value::String(url.clone());
    }
    if let Some(ref key) = agent.api_key {
        obj["apiKey"] = serde_json::Value::String(key.clone());
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&obj).unwrap_or_default());
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
