use crate::theme::Config;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

pub(super) fn opencode_command(config: &Config) -> String {
    config
        .agents
        .iter()
        .find(|agent| agent.name == "opencode")
        .map(|agent| agent.cmd.trim().to_string())
        .filter(|cmd| !cmd.is_empty())
        .unwrap_or_else(default_opencode_command)
}

pub(in crate::app::actions) fn command_with_args<'a>(
    command: &str,
    args: impl IntoIterator<Item = &'a str>,
) -> String {
    let mut command_line = command.trim().to_string();
    for arg in args {
        command_line.push(' ');
        command_line.push_str(&crate::shell_quote::single_quote(arg));
    }
    command_line
}

pub(in crate::app::actions) fn run_with_args(
    command: &str,
    args: &[&str],
    cwd: Option<&Path>,
) -> io::Result<Output> {
    let command_line = command_with_args(command, args.iter().copied());
    let mut process = Command::new("/bin/sh");
    process.args(["-lc", &command_line]);
    if let Some(cwd) = cwd {
        process.current_dir(cwd);
    }
    process.output()
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

fn default_opencode_command() -> String {
    let home_bin = crate::paths::pad_home_dir()
        .parent()
        .map(|home| home.join(".opencode").join("bin").join("opencode"));
    if let Some(path) = home_bin.filter(|path| path.exists()) {
        crate::shell_quote::single_quote(&path.to_string_lossy())
    } else {
        "opencode".to_string()
    }
}
