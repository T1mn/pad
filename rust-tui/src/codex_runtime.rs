use serde_json::Value;
use std::io;

const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";

pub fn prepare_agent_command(agent_name: &str, agent_cmd: &str) -> io::Result<String> {
    if !is_codex_agent(agent_name, agent_cmd) {
        return Ok(agent_cmd.to_string());
    }

    crate::paths::ensure_pad_codex_home_layout()?;
    crate::paths::ensure_pad_codex_wrapper()?;
    ensure_pad_codex_auth_ready()?;
    Ok(with_pad_codex_runtime(agent_cmd))
}

pub fn with_pad_codex_runtime(agent_cmd: &str) -> String {
    let rest = codex_args_without_profile(agent_cmd);
    let wrapper = shell_single_quote(&crate::paths::pad_codex_wrapper_path().to_string_lossy());
    if rest.trim().is_empty() {
        wrapper
    } else {
        format!("{wrapper} {rest}")
    }
}

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

fn codex_args_without_profile(agent_cmd: &str) -> String {
    let cmd = agent_cmd.trim();
    let cmd = if cmd.is_empty() { "codex" } else { cmd };
    let cmd = strip_profile_args(cmd);

    split_first_token(&cmd)
        .map(|(_, rest)| rest.trim_start().to_string())
        .unwrap_or_default()
}

fn strip_profile_args(command: &str) -> String {
    let mut out = Vec::new();
    let mut skip_next = false;
    for token in command.split_whitespace() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if token == "--profile" || token == "-p" {
            skip_next = true;
            continue;
        }
        if token.starts_with("--profile=") {
            continue;
        }
        out.push(token);
    }
    out.join(" ")
}

fn split_first_token(command: &str) -> Option<(&str, &str)> {
    let trimmed = command.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.find(char::is_whitespace) {
        Some(index) => Some((&trimmed[..index], &trimmed[index..])),
        None => Some((trimmed, "")),
    }
}

fn is_codex_agent(agent_name: &str, agent_cmd: &str) -> bool {
    agent_name.trim() == "codex" || first_command_token(agent_cmd) == Some("codex")
}

fn first_command_token(command: &str) -> Option<&str> {
    command.split_whitespace().next().map(|token| {
        token
            .rsplit_once('/')
            .map(|(_, basename)| basename)
            .unwrap_or(token)
    })
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

pub(crate) fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_home(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("pad-codex-runtime-{name}-{stamp}"))
    }

    fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
        let _guard = crate::test_support::home_env_lock()
            .lock()
            .expect("lock codex runtime tests");
        let home = temp_home(name);
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("create temp home");

        let prev_home = std::env::var_os("HOME");
        let prev_openai_api_key = std::env::var_os(OPENAI_API_KEY_ENV);
        std::env::set_var("HOME", &home);
        std::env::remove_var(OPENAI_API_KEY_ENV);

        let result = f(&home);

        if let Some(prev) = prev_home {
            std::env::set_var("HOME", prev);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(prev) = prev_openai_api_key {
            std::env::set_var(OPENAI_API_KEY_ENV, prev);
        } else {
            std::env::remove_var(OPENAI_API_KEY_ENV);
        }
        let _ = std::fs::remove_dir_all(&home);

        result
    }

    #[test]
    fn codex_agent_command_uses_pad_profile_without_codex_home() {
        with_temp_home("profile", |_home| {
            let command = with_pad_codex_runtime("codex --model gpt-5");
            assert!(command.starts_with("'"));
            assert!(command.contains("/.pad/scripts/pad-codex'"));
            assert!(command.ends_with(" --model gpt-5"));
            assert!(!command.contains("OPENAI_API_KEY="));
        });
    }

    #[test]
    fn codex_agent_command_replaces_existing_profile_with_pad() {
        with_temp_home("replace-profile", |_home| {
            let command = with_pad_codex_runtime("codex --profile work --model gpt-5");

            assert!(command.contains("/.pad/scripts/pad-codex' --model gpt-5"));
            assert!(!command.contains("--profile work"));
        });
    }

    #[test]
    fn codex_agent_command_uses_wrapper_instead_of_inlining_auth() {
        with_temp_home("auth", |_home| {
            let auth_path = crate::paths::pad_codex_auth_path();
            std::fs::create_dir_all(auth_path.parent().expect("auth parent"))
                .expect("create auth parent");
            std::fs::write(&auth_path, r#"{"OPENAI_API_KEY":"sk-test'1"}"#).expect("write auth");

            let command = with_pad_codex_runtime("codex");

            assert!(command.contains("/.pad/scripts/pad-codex'"));
            assert!(!command.contains("sk-test"));
        });
    }

    #[test]
    fn codex_prepare_fails_when_pad_provider_requires_auth_but_key_is_missing() {
        with_temp_home("missing-auth", |_home| {
            let config_path = crate::paths::pad_codex_config_path();
            std::fs::create_dir_all(config_path.parent().expect("config parent"))
                .expect("create config parent");
            std::fs::write(
                &config_path,
                r#"
model_provider = "local"

[model_providers.local]
base_url = "http://localhost:8317/v1"
name = "local"
requires_openai_auth = true
"#,
            )
            .expect("write config");

            let err = prepare_agent_command("codex", "codex").expect_err("missing key");
            assert!(err.to_string().contains("needs relay auth"));
        });
    }

    #[test]
    fn codex_prepare_allows_required_auth_when_pad_key_exists() {
        with_temp_home("ready-auth", |_home| {
            let config_path = crate::paths::pad_codex_config_path();
            std::fs::create_dir_all(config_path.parent().expect("config parent"))
                .expect("create config parent");
            std::fs::write(
                &config_path,
                r#"
model_provider = "local"

[model_providers.local]
base_url = "http://localhost:8317/v1"
name = "local"
requires_openai_auth = true
"#,
            )
            .expect("write config");
            let auth_path = crate::paths::pad_codex_auth_path();
            std::fs::create_dir_all(auth_path.parent().expect("auth parent"))
                .expect("create auth parent");
            std::fs::write(&auth_path, r#"{"OPENAI_API_KEY":"local-key"}"#).expect("write auth");

            let command = prepare_agent_command("codex", "codex --model gpt-5").expect("ready");
            assert!(command.contains("/.pad/scripts/pad-codex' --model gpt-5"));
        });
    }

    #[test]
    fn first_command_token_accepts_absolute_codex_path() {
        assert_eq!(
            first_command_token("/opt/bin/codex --version"),
            Some("codex")
        );
    }

    #[test]
    fn non_codex_agent_is_not_wrapped() {
        assert!(!is_codex_agent("claude", "claude"));
    }
}
