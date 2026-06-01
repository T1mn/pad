use crate::theme::AgentConfig;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) fn should_restore_standard_relay_config(agent: &AgentConfig) -> bool {
    let Some(prov) = agent.active() else {
        return true;
    };
    prov.base_url.trim().is_empty() || prov.api_key.trim().is_empty()
}

pub(super) fn opencode_config_path() -> PathBuf {
    if let Some(path) = std::env::var_os("OPENCODE_CONFIG") {
        return PathBuf::from(path);
    }

    if let Some(dir) = std::env::var_os("OPENCODE_CONFIG_DIR") {
        let dir = PathBuf::from(dir);
        return existing_opencode_config_in(&dir).unwrap_or_else(|| dir.join("opencode.jsonc"));
    }

    let dir = home_dir().join(".config").join("opencode");
    existing_opencode_config_in(&dir).unwrap_or_else(|| dir.join("opencode.jsonc"))
}

fn existing_opencode_config_in(dir: &Path) -> Option<PathBuf> {
    let jsonc = dir.join("opencode.jsonc");
    if jsonc.exists() {
        return Some(jsonc);
    }
    let json = dir.join("opencode.json");
    json.exists().then_some(json)
}

pub(super) fn opencode_managed_state_path() -> PathBuf {
    crate::paths::pad_home_dir().join("opencode-relay-state.json")
}

pub(super) fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(super) fn codex_permission_state_path() -> PathBuf {
    crate::paths::pad_home_dir().join("codex-permissions-state.json")
}

pub(super) fn claude_permission_state_path() -> PathBuf {
    crate::paths::pad_home_dir().join("claude-permissions-state.json")
}

pub(super) fn claude_settings_path() -> PathBuf {
    home_dir().join(".claude").join("settings.json")
}

pub(super) fn claude_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("claude-settings.pre-pad.json")
}

pub(super) fn gemini_settings_path() -> PathBuf {
    home_dir().join(".gemini").join("settings.json")
}

pub(super) fn gemini_env_path() -> PathBuf {
    home_dir().join(".gemini").join(".env")
}

pub(super) fn gemini_settings_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("gemini-settings.pre-pad.json")
}

pub(super) fn gemini_env_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("gemini-env.pre-pad")
}

pub(super) fn codex_config_path() -> PathBuf {
    crate::paths::pad_codex_config_path()
}

pub(super) fn codex_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("codex-config.pre-pad.toml")
}

pub(super) fn codex_auth_path() -> PathBuf {
    crate::paths::pad_codex_auth_path()
}

pub(super) fn codex_auth_backup_path() -> PathBuf {
    crate::paths::pad_home_dir().join("codex-auth.pre-pad.json")
}

pub(super) fn has_backup(path: &Path) -> bool {
    path.exists()
}

pub(super) fn backup_file(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

pub(super) fn restore_file(path: &Path, backup_path: &Path) {
    let Ok(content) = std::fs::read_to_string(backup_path) else {
        return;
    };
    write_text_file(path, &content);
}

pub(super) fn write_text_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, content);
}

pub(super) fn read_json_value(path: &Path, fallback: serde_json::Value) -> serde_json::Value {
    let Ok(content) = std::fs::read_to_string(path) else {
        return fallback;
    };
    let parsed = serde_json::from_str::<serde_json::Value>(&strip_json_comments(&content))
        .unwrap_or(fallback);
    if parsed.is_object() {
        parsed
    } else {
        json!({})
    }
}

fn strip_json_comments(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if next == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut previous = '\0';
                    for next in chars.by_ref() {
                        if previous == '*' && next == '/' {
                            break;
                        }
                        previous = next;
                    }
                    continue;
                }
                _ => {}
            }
        }

        out.push(ch);
    }

    out
}

pub(super) fn write_json_value(path: &Path, value: &serde_json::Value) {
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

pub(super) fn parse_json_object(content: &str) -> serde_json::Value {
    let mut obj = serde_json::from_str::<serde_json::Value>(content).unwrap_or_else(|_| json!({}));
    if !obj.is_object() {
        obj = json!({});
    }
    obj
}

pub(super) fn serialize_json_pretty(value: &serde_json::Value) -> String {
    let mut serialized = serde_json::to_string_pretty(value).unwrap_or_default();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    serialized
}

pub(super) fn parse_toml_document(content: &str) -> toml::Value {
    content
        .parse::<toml::Value>()
        .unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
}

pub(super) fn serialize_toml_document(value: &toml::Value) -> String {
    let mut serialized = toml::to_string(value).unwrap_or_default();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    serialized
}

pub(super) fn parse_env_file(content: &str) -> BTreeMap<String, String> {
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

pub(super) fn serialize_env_file(map: &BTreeMap<String, String>) -> String {
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

pub(super) fn has_pad_codex_backup() -> bool {
    codex_backup_path().exists()
}

pub(super) fn has_pad_codex_auth_backup() -> bool {
    codex_auth_backup_path().exists()
}

pub(super) fn backup_codex_config(content: &str) -> std::io::Result<()> {
    let backup = codex_backup_path();
    if let Some(parent) = backup.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(backup, content)
}

pub(super) fn backup_codex_auth(content: &str) -> std::io::Result<()> {
    let backup = codex_auth_backup_path();
    if let Some(parent) = backup.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(backup, content)
}

pub(super) fn restore_codex_config() {
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

pub(super) fn restore_codex_auth() {
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
