mod command {
    use std::ffi::OsString;
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub(super) fn install_github_agent(cwd: &Path, command: &OsString) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["new-window", "-c"])
            .arg(cwd)
            .arg(github_install_command(command))
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "tmux new-window exited with {status}"
            )))
        }
    }

    pub(in crate::app::actions) fn github_install_command(command: &OsString) -> String {
        format!(
            "{} github install",
            crate::codex_runtime::shell_single_quote(&command.to_string_lossy())
        )
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn github_started_title(locale: Locale) -> &'static str {
        localized(
            locale,
            "OpenCode GitHub install 已启动",
            "OpenCode GitHub Install Started",
        )
    }

    pub(super) fn github_failed_title(locale: Locale) -> &'static str {
        localized(
            locale,
            "OpenCode GitHub install 失败",
            "OpenCode GitHub Install Failed",
        )
    }
}

use super::*;
use std::path::PathBuf;

impl App {
    pub fn install_opencode_github_agent(&mut self) -> bool {
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match command::install_github_agent(&cwd, &opencode_cli::opencode_command(&self.config)) {
            Ok(()) => {
                self.show_action_toast(
                    text::github_started_title(self.locale),
                    &cwd.display().to_string(),
                );
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(text::github_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

#[cfg(test)]
pub(in crate::app::actions) use command::github_install_command;

#[cfg(test)]
#[path = "opencode_github_tests.rs"]
mod opencode_github_tests;
