use super::types::CodexCliVersionInfo;
use std::process::Command;

pub(super) fn detect_codex_cli_version_info() -> CodexCliVersionInfo {
    let binary_path = command_path("codex");
    let local_version = command_output("codex", &["--version"]).and_then(|raw| parse_version(&raw));
    let latest_version = command_output("npm", &["view", "@openai/codex", "version", "--json"])
        .and_then(|raw| parse_json_string(&raw));

    CodexCliVersionInfo {
        binary_path,
        local_version,
        latest_version,
    }
}

pub(super) fn update_codex_cli() -> Result<CodexCliVersionInfo, String> {
    let binary_path = command_path("codex")
        .ok_or_else(|| "Codex executable was not found in PATH".to_string())?;
    run_codex_cli_update(&binary_path)?;

    Ok(detect_codex_cli_version_info())
}

fn run_codex_cli_update(binary_path: &str) -> Result<(), String> {
    let output = Command::new(binary_path)
        .arg("update")
        .output()
        .map_err(|err| format!("failed to launch Codex update: {err}"))?;

    if !output.status.success() {
        return Err(failed_command_message(&output));
    }

    Ok(())
}

fn failed_command_message(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("Codex update exited with status {}", output.status)
    }
}

fn command_path(name: &str) -> Option<String> {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {name} 2>/dev/null"))
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_version(raw: &str) -> Option<String> {
    raw.split_whitespace()
        .rev()
        .find(|token| token.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
        .map(|token| token.to_string())
}

fn parse_json_string(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('"').trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
