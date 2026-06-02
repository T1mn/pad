use serde_json::Value;
use std::io;

const PAD_CODEX_HOOKS_ENV: &str = "PAD_CODEX_HOOKS";
const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";

pub fn prepare_agent_command(agent_name: &str, agent_cmd: &str) -> io::Result<String> {
    if !is_codex_agent(agent_name, agent_cmd) {
        return Ok(agent_cmd.to_string());
    }

    crate::paths::ensure_pad_codex_home_layout()?;
    Ok(with_pad_codex_runtime(agent_cmd))
}

pub fn with_pad_codex_runtime(agent_cmd: &str) -> String {
    let command = with_pad_profile(agent_cmd);
    let mut env_parts = vec![format!("{PAD_CODEX_HOOKS_ENV}=1")];
    if let Some(api_key) = pad_codex_openai_api_key() {
        env_parts.push(format!(
            "{OPENAI_API_KEY_ENV}={}",
            shell_single_quote(&api_key)
        ));
    }
    format!("env {} {}", env_parts.join(" "), command)
}

fn with_pad_profile(agent_cmd: &str) -> String {
    let cmd = agent_cmd.trim();
    let cmd = if cmd.is_empty() { "codex" } else { cmd };
    let cmd = strip_profile_args(cmd);

    let Some((first, rest)) = split_first_token(&cmd) else {
        return format!("codex --profile {}", crate::paths::pad_codex_profile());
    };
    let rest = rest.trim_start();
    if rest.is_empty() {
        format!("{first} --profile {}", crate::paths::pad_codex_profile())
    } else {
        format!(
            "{first} --profile {} {rest}",
            crate::paths::pad_codex_profile()
        )
    }
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
    fn codex_agent_command_uses_pad_profile_without_codex_home() {
        with_temp_home("profile", |_home| {
            let command = with_pad_codex_runtime("codex --model gpt-5");
            assert!(command.starts_with("env PAD_CODEX_HOOKS=1 codex --profile pad"));
            assert!(command.ends_with(" --model gpt-5"));
            assert!(!command.contains("CODEX_HOME="));
        });
    }

    #[test]
    fn codex_agent_command_replaces_existing_profile_with_pad() {
        with_temp_home("replace-profile", |_home| {
            let command = with_pad_codex_runtime("codex --profile work --model gpt-5");

            assert!(command.contains(" codex --profile pad --model gpt-5"));
            assert!(!command.contains("--profile work"));
        });
    }

    #[test]
    fn codex_agent_command_injects_pad_auth_as_environment_only() {
        with_temp_home("auth", |_home| {
            let auth_path = crate::paths::pad_codex_auth_path();
            std::fs::create_dir_all(auth_path.parent().expect("auth parent"))
                .expect("create auth parent");
            std::fs::write(&auth_path, r#"{"OPENAI_API_KEY":"sk-test'1"}"#).expect("write auth");

            let command = with_pad_codex_runtime("codex");

            assert!(command.contains("OPENAI_API_KEY='sk-test'\\''1'"));
            assert!(command.contains(" codex --profile pad"));
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
