mod command;
mod text;

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
