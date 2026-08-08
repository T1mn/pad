mod command {
    use std::ffi::OsString;
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub(super) fn attach_opencode_server(
        url: &str,
        cwd: &Path,
        command: &OsString,
    ) -> io::Result<()> {
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

    pub(in crate::app::actions) fn attach_command(url: &str, command: &OsString) -> String {
        format!(
            "{} attach {}",
            crate::codex_runtime::shell_single_quote(&command.to_string_lossy()),
            crate::codex_runtime::shell_single_quote(url)
        )
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn attach_saved_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode 已 attach", "OpenCode Attached")
    }

    pub(super) fn attach_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode attach 失败", "OpenCode Attach Failed")
    }
}
mod url {
    use super::super::helpers::trim_wrapping_quotes;

    pub(in crate::app::actions) fn normalize_server_url(
        text: &str,
    ) -> Result<String, &'static str> {
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

    fn is_http_url(value: &str) -> bool {
        let rest = value
            .strip_prefix("http://")
            .or_else(|| value.strip_prefix("https://"));
        rest.is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
    }
}

use super::*;
use std::path::PathBuf;

impl App {
    pub fn attach_opencode_from_clipboard(&mut self) -> bool {
        let url = match url_from_clipboard() {
            Ok(url) => url,
            Err(message) => {
                self.show_action_toast(text::attach_failed_title(self.locale), &message);
                return false;
            }
        };
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match command::attach_opencode_server(
            &url,
            &cwd,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(()) => {
                self.show_action_toast(text::attach_saved_title(self.locale), &url);
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(text::attach_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn url_from_clipboard() -> Result<String, String> {
    let text = crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
    url::normalize_server_url(&text).map_err(str::to_string)
}

#[cfg(test)]
pub(in crate::app::actions) use command::attach_command;
#[cfg(test)]
pub(in crate::app::actions) use url::normalize_server_url;
