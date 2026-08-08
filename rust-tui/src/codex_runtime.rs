use std::io;

mod auth {
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
}
mod command {
    pub fn with_pad_codex_runtime(agent_cmd: &str) -> String {
        let rest = codex_args_without_profile(agent_cmd);
        let wrapper = shell_single_quote(&crate::paths::pad_codex_wrapper_path().to_string_lossy());
        if rest.trim().is_empty() {
            wrapper
        } else {
            format!("{wrapper} {rest}")
        }
    }

    pub fn with_pad_claude_runtime(agent_cmd: &str) -> String {
        let cmd = agent_cmd.trim();
        let cmd = if cmd.is_empty() { "claude" } else { cmd };
        format!(
            "env -u ANTHROPIC_BASE_URL -u ANTHROPIC_API_KEY -u ANTHROPIC_AUTH_TOKEN -u ANTHROPIC_MODEL -u ANTHROPIC_CUSTOM_MODEL_OPTION {cmd}"
        )
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
        let mut out = String::new();
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
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(token);
        }
        out
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

    pub(super) fn is_codex_agent(agent_name: &str, agent_cmd: &str) -> bool {
        agent_name.trim() == "codex" || first_command_token(agent_cmd) == Some("codex")
    }

    pub(super) fn is_claude_agent(agent_name: &str, agent_cmd: &str) -> bool {
        let name = agent_name.trim();
        if name == "deepseek" || name == "deepseek(cc)" {
            return false;
        }
        name == "claude" || first_command_token(agent_cmd) == Some("claude")
    }

    pub(super) fn first_command_token(command: &str) -> Option<&str> {
        command.split_whitespace().next().map(|token| {
            token
                .rsplit_once('/')
                .map(|(_, basename)| basename)
                .unwrap_or(token)
        })
    }

    pub(crate) fn shell_single_quote(value: &str) -> String {
        crate::shell_quote::single_quote(value)
    }
}
#[cfg(test)]
mod tests;

pub use auth::ensure_pad_codex_auth_ready;
pub(crate) use command::shell_single_quote;
pub use command::with_pad_codex_runtime;

pub fn prepare_agent_command(agent_name: &str, agent_cmd: &str) -> io::Result<String> {
    if command::is_claude_agent(agent_name, agent_cmd) {
        return Ok(command::with_pad_claude_runtime(agent_cmd));
    }

    if !command::is_codex_agent(agent_name, agent_cmd) {
        return Ok(agent_cmd.to_string());
    }

    crate::paths::ensure_pad_codex_home_layout()?;
    crate::paths::ensure_pad_codex_wrapper()?;
    ensure_pad_codex_auth_ready()?;
    Ok(with_pad_codex_runtime(agent_cmd))
}
