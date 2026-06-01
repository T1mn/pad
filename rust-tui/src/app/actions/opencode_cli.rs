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
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        "session".to_string()
    } else {
        out.chars().take(96).collect()
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
mod tests {
    use super::{first_command_token, safe_filename};

    #[test]
    fn opencode_command_uses_first_configured_token() {
        assert_eq!(
            first_command_token("/opt/bin/opencode --pure"),
            "/opt/bin/opencode"
        );
    }

    #[test]
    fn safe_filename_sanitizes_and_falls_back() {
        assert_eq!(safe_filename("ses/../abc def"), "ses_abc_def");
        assert_eq!(safe_filename("***"), "session");
    }
}
