use crate::app::{App, CodexCliVersionInfo};
use std::process::Command;
use std::thread;
use tokio::sync::mpsc;

impl App {
    pub fn trigger_codex_cli_version_check(&mut self) {
        if self.codex_cli_check_in_progress {
            return;
        }

        self.codex_cli_check_in_progress = true;
        let (tx, rx) = mpsc::channel(1);
        self.codex_cli_check_rx = Some(rx);
        self.dirty = true;

        thread::spawn(move || {
            let _ = tx.blocking_send(detect_codex_cli_version_info());
        });
    }

    pub fn check_codex_cli_version_result(&mut self) {
        if let Some(ref mut rx) = self.codex_cli_check_rx {
            match rx.try_recv() {
                Ok(info) => {
                    self.codex_cli_version_info = Some(info);
                    self.codex_cli_check_in_progress = false;
                    self.codex_cli_check_rx = None;
                    self.dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.codex_cli_check_in_progress = false;
                    self.codex_cli_check_rx = None;
                    self.dirty = true;
                }
            }
        }
    }

    pub fn trigger_codex_cli_update(&mut self) {
        if self.codex_cli_update_in_progress {
            return;
        }

        self.codex_cli_update_in_progress = true;
        let (tx, rx) = mpsc::channel(1);
        self.codex_cli_update_rx = Some(rx);
        self.dirty = true;

        thread::spawn(move || {
            let _ = tx.blocking_send(update_codex_cli());
        });
    }

    pub fn check_codex_cli_update_result(&mut self) {
        if let Some(ref mut rx) = self.codex_cli_update_rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.codex_cli_update_in_progress = false;
                    self.codex_cli_update_rx = None;
                    match result {
                        Ok(info) => {
                            self.codex_cli_version_info = Some(info.clone());
                            let local = info.local_version.unwrap_or_else(|| "?".to_string());
                            let latest = info.latest_version.unwrap_or_else(|| "?".to_string());
                            if matches!(
                                self.locale,
                                crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
                            ) {
                                self.show_action_toast(
                                    "Codex 升级完成",
                                    &format!("当前 {local} · 最新 {latest}"),
                                );
                            } else {
                                self.show_action_toast(
                                    "Codex updated",
                                    &format!("Current {local} · latest {latest}"),
                                );
                            }
                        }
                        Err(err) => {
                            if matches!(
                                self.locale,
                                crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
                            ) {
                                self.show_action_toast("Codex 升级失败", &err);
                            } else {
                                self.show_action_toast("Codex update failed", &err);
                            }
                        }
                    }
                    self.dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.codex_cli_update_in_progress = false;
                    self.codex_cli_update_rx = None;
                    self.dirty = true;
                }
            }
        }
    }
}

fn detect_codex_cli_version_info() -> CodexCliVersionInfo {
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

fn update_codex_cli() -> Result<CodexCliVersionInfo, String> {
    let output = Command::new("npm")
        .args(["install", "-g", "@openai/codex@latest"])
        .output()
        .map_err(|err| format!("failed to launch npm: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("npm exited with status {}", output.status)
        };
        return Err(message);
    }

    Ok(detect_codex_cli_version_info())
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
mod tests {
    use super::{parse_json_string, parse_version};

    #[test]
    fn parse_codex_version_from_cli_output() {
        assert_eq!(
            parse_version("codex-cli 0.125.0"),
            Some("0.125.0".to_string())
        );
    }

    #[test]
    fn parse_npm_version_json_output() {
        assert_eq!(
            parse_json_string("\"0.125.0\""),
            Some("0.125.0".to_string())
        );
    }
}
