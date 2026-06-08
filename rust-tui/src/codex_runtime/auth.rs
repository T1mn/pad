use serde_json::Value;
use std::io;

const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";

pub fn ensure_pad_codex_auth_ready() -> io::Result<()> {
    if !pad_profile_requires_openai_auth() {
        return Ok(());
    }
    if pad_codex_openai_api_key().is_some() || std::env::var_os(OPENAI_API_KEY_ENV).is_some() {
        return Ok(());
    }

    Err(io::Error::other(format!(
        "Codex pad profile needs relay auth, but {OPENAI_API_KEY_ENV} is missing and {} has no key",
        crate::paths::pad_codex_auth_path().display()
    )))
}

fn pad_codex_openai_api_key() -> Option<String> {
    let content = std::fs::read_to_string(crate::paths::pad_codex_auth_path()).ok()?;
    let value = serde_json::from_str::<Value>(&content).ok()?;
    value
        .get(OPENAI_API_KEY_ENV)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
}

fn pad_profile_requires_openai_auth() -> bool {
    let content = match std::fs::read_to_string(crate::paths::pad_codex_config_path()) {
        Ok(content) => content,
        Err(_) => return false,
    };
    let doc = match content.parse::<toml::Value>() {
        Ok(doc) => doc,
        Err(_) => return false,
    };
    let Some(provider_name) = doc.get("model_provider").and_then(toml::Value::as_str) else {
        return false;
    };
    doc.get("model_providers")
        .and_then(toml::Value::as_table)
        .and_then(|providers| providers.get(provider_name))
        .and_then(toml::Value::as_table)
        .and_then(|provider| provider.get("requires_openai_auth"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false)
}

#[cfg(test)]
pub(super) const TEST_OPENAI_API_KEY_ENV: &str = OPENAI_API_KEY_ENV;
