mod command {
    use std::ffi::OsString;
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub(super) fn open_opencode_pr(
        pr_number: &str,
        cwd: &Path,
        command: &OsString,
    ) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["new-window", "-c"])
            .arg(cwd)
            .arg(pr_command(pr_number, command))
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "tmux new-window exited with {status}"
            )))
        }
    }

    pub(in crate::app::actions) fn pr_command(pr_number: &str, command: &OsString) -> String {
        format!(
            "{} pr {}",
            crate::codex_runtime::shell_single_quote(&command.to_string_lossy()),
            pr_number
        )
    }
}
mod parse {
    use super::super::helpers::trim_wrapping_quotes;

    pub(in crate::app::actions) fn normalize_pr_number(text: &str) -> Result<String, &'static str> {
        let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
        let Some(first) = lines.next() else {
            return Err("Clipboard is empty");
        };
        if lines.next().is_some() {
            return Err("Clipboard must contain one PR number or URL");
        }
        let value = trim_wrapping_quotes(first);
        let candidate = value
            .strip_prefix('#')
            .or_else(|| number_after_pull_segment(value))
            .unwrap_or(value);
        if is_positive_integer(candidate) {
            Ok(candidate.to_string())
        } else {
            Err("Clipboard must contain a GitHub PR number or /pull/<number> URL")
        }
    }

    fn number_after_pull_segment(value: &str) -> Option<&str> {
        let marker = "/pull/";
        let start = value.find(marker)? + marker.len();
        let tail = &value[start..];
        let len = tail
            .char_indices()
            .find_map(|(idx, ch)| (!ch.is_ascii_digit()).then_some(idx))
            .unwrap_or(tail.len());
        (len > 0).then_some(&tail[..len])
    }

    fn is_positive_integer(value: &str) -> bool {
        !value.is_empty() && value != "0" && value.chars().all(|ch| ch.is_ascii_digit())
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn pr_started_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode PR 已启动", "OpenCode PR Started")
    }

    pub(super) fn pr_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode PR 失败", "OpenCode PR Failed")
    }
}

use super::*;
use std::path::PathBuf;

impl App {
    pub fn open_opencode_pr_from_clipboard(&mut self) -> bool {
        let pr_number = match pr_number_from_clipboard() {
            Ok(number) => number,
            Err(message) => {
                self.show_action_toast(text::pr_failed_title(self.locale), &message);
                return false;
            }
        };

        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match command::open_opencode_pr(
            &pr_number,
            &cwd,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(()) => {
                self.show_action_toast(
                    text::pr_started_title(self.locale),
                    &format!("PR #{pr_number}"),
                );
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(text::pr_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn pr_number_from_clipboard() -> Result<String, String> {
    let text = crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
    parse::normalize_pr_number(&text).map_err(str::to_string)
}

#[cfg(test)]
pub(in crate::app::actions) use command::pr_command;
#[cfg(test)]
pub(in crate::app::actions) use parse::normalize_pr_number;

#[cfg(test)]
#[path = "opencode_pr_tests.rs"]
mod opencode_pr_tests;
