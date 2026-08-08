mod command {
    use std::ffi::OsString;
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub(super) fn serve_opencode(cwd: &Path, command: &OsString) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["new-window", "-c"])
            .arg(cwd)
            .arg(serve_command(command))
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "tmux new-window exited with {status}"
            )))
        }
    }

    pub(in crate::app::actions) fn serve_command(command: &OsString) -> String {
        format!(
            "{} serve --hostname 127.0.0.1 --port 0",
            crate::codex_runtime::shell_single_quote(&command.to_string_lossy())
        )
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn serve_started_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode serve 已启动", "OpenCode Serve Started")
    }

    pub(super) fn serve_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode serve 失败", "OpenCode Serve Failed")
    }
}

use super::*;
use std::path::PathBuf;

impl App {
    pub fn serve_opencode_for_selected_thread(&mut self) -> bool {
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match command::serve_opencode(&cwd, &opencode_cli::opencode_command(&self.config)) {
            Ok(()) => {
                self.show_action_toast(
                    text::serve_started_title(self.locale),
                    &cwd.display().to_string(),
                );
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(text::serve_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

#[cfg(test)]
pub(in crate::app::actions) use command::serve_command;
