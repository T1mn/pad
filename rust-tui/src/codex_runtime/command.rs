pub fn with_pad_codex_runtime(agent_cmd: &str) -> String {
    let rest = codex_args_without_profile(agent_cmd);
    let wrapper = shell_single_quote(&crate::paths::pad_codex_wrapper_path().to_string_lossy());
    if rest.trim().is_empty() {
        wrapper
    } else {
        format!("{wrapper} {rest}")
    }
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

pub(super) fn first_command_token(command: &str) -> Option<&str> {
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
