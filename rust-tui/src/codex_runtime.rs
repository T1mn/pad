use std::io;

pub fn prepare_agent_command(agent_name: &str, agent_cmd: &str) -> io::Result<String> {
    if !is_codex_agent(agent_name, agent_cmd) {
        return Ok(agent_cmd.to_string());
    }

    crate::paths::ensure_pad_codex_home_layout()?;
    Ok(with_codex_home(agent_cmd))
}

pub fn with_codex_home(agent_cmd: &str) -> String {
    let cmd = agent_cmd.trim();
    let cmd = if cmd.is_empty() { "codex" } else { cmd };
    format!(
        "env CODEX_HOME={} {}",
        shell_single_quote(&crate::paths::pad_codex_home_dir().to_string_lossy()),
        cmd
    )
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

pub(crate) fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_agent_command_gets_codex_home_prefix() {
        let command = with_codex_home("codex --profile work");
        assert!(command.starts_with("env CODEX_HOME='"));
        assert!(
            command.ends_with("' codex --profile work")
                || command.contains("' codex --profile work")
        );
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
