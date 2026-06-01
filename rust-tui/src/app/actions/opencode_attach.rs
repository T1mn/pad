use super::*;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Command;

impl App {
    pub fn attach_opencode_from_clipboard(&mut self) -> bool {
        let url = match crate::app::clipboard::read_text_from_clipboard() {
            Ok(text) => match normalize_server_url(&text) {
                Ok(url) => url,
                Err(message) => {
                    self.show_action_toast(attach_failed_title(self.locale), message);
                    return false;
                }
            },
            Err(err) => {
                self.show_action_toast(attach_failed_title(self.locale), &err.to_string());
                return false;
            }
        };
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match attach_opencode_server(&url, &cwd, &opencode_cli::opencode_command(&self.config)) {
            Ok(()) => {
                self.show_action_toast(attach_saved_title(self.locale), &url);
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(attach_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn attach_opencode_server(url: &str, cwd: &PathBuf, command: &OsString) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(attach_command(url, command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

fn attach_command(url: &str, command: &OsString) -> String {
    format!(
        "{} attach {}",
        shell_single_quote(&command.to_string_lossy()),
        shell_single_quote(url)
    )
}

fn normalize_server_url(text: &str) -> Result<String, &'static str> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first) = lines.next() else {
        return Err("Clipboard is empty");
    };
    if lines.next().is_some() {
        return Err("Clipboard must contain one OpenCode server URL");
    }
    let url = trim_wrapping_quotes(first).trim_end_matches('/');
    if is_http_url(url) && !url.contains(char::is_whitespace) {
        Ok(url.to_string())
    } else {
        Err("Clipboard must contain an http(s) OpenCode server URL")
    }
}

fn trim_wrapping_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn is_http_url(value: &str) -> bool {
    let rest = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"));
    rest.is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
}

fn shell_single_quote(value: &str) -> String {
    crate::codex_runtime::shell_single_quote(value)
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

fn attach_saved_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 已 attach"
    } else {
        "OpenCode Attached"
    }
}

fn attach_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode attach 失败"
    } else {
        "OpenCode Attach Failed"
    }
}

#[cfg(test)]
#[path = "opencode_attach_tests.rs"]
mod opencode_attach_tests;
