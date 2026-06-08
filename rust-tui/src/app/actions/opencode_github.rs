mod command;
mod text;

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
