mod command {
    use std::ffi::OsString;
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub(super) fn open_opencode_web(cwd: &Path, command: &OsString) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["new-window", "-c"])
            .arg(cwd)
            .arg(web_command(command))
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "tmux new-window exited with {status}"
            )))
        }
    }

    pub(in crate::app::actions) fn web_command(command: &OsString) -> String {
        format!(
            "{} web",
            crate::codex_runtime::shell_single_quote(&command.to_string_lossy())
        )
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn web_opened_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode Web 已打开", "OpenCode Web Opened")
    }

    pub(super) fn web_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode Web 失败", "OpenCode Web Failed")
    }
}

use super::*;
use std::path::PathBuf;

impl App {
    pub fn open_opencode_web_for_selected_thread(&mut self) -> bool {
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match command::open_opencode_web(&cwd, &opencode_cli::opencode_command(&self.config)) {
            Ok(()) => {
                self.show_action_toast(
                    text::web_opened_title(self.locale),
                    &cwd.display().to_string(),
                );
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(text::web_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

#[cfg(test)]
pub(in crate::app::actions) use command::web_command;

#[cfg(test)]
#[path = "opencode_web_tests.rs"]
mod opencode_web_tests;
