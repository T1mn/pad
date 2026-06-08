mod command;
mod text;

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

#[cfg(test)]
#[path = "opencode_serve_tests.rs"]
mod opencode_serve_tests;
