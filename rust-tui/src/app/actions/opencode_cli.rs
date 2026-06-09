use crate::theme::Config;
use std::ffi::OsString;

pub(super) fn opencode_command(config: &Config) -> OsString {
    config
        .agents
        .iter()
        .find(|agent| agent.name == "opencode")
        .map(|agent| first_command_token(&agent.cmd))
        .filter(|cmd| !cmd.is_empty())
        .map(OsString::from)
        .unwrap_or_else(default_opencode_command)
}

pub(super) fn safe_filename(value: &str) -> String {
    let mut out = String::with_capacity(value.len().min(96));
    let mut sanitized_len = 0usize;
    let mut last_was_underscore = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            if sanitized_len == 0 && ch == '_' {
                continue;
            }
            if out.len() < 96 {
                out.push(ch);
            }
            sanitized_len += 1;
            last_was_underscore = ch == '_';
        } else if sanitized_len > 0 && !last_was_underscore {
            if out.len() < 96 {
                out.push('_');
            }
            sanitized_len += 1;
            last_was_underscore = true;
        }
    }
    if sanitized_len <= 96 {
        while out.ends_with('_') {
            out.pop();
        }
    }
    if out.is_empty() {
        "session".to_string()
    } else {
        out
    }
}

fn first_command_token(command: &str) -> String {
    command
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn default_opencode_command() -> OsString {
    let home_bin = crate::paths::pad_home_dir()
        .parent()
        .map(|home| home.join(".opencode").join("bin").join("opencode"));
    if let Some(path) = home_bin.filter(|path| path.exists()) {
        path.into_os_string()
    } else {
        OsString::from("opencode")
    }
}

#[cfg(test)]
#[path = "opencode_cli_tests.rs"]
mod tests;
