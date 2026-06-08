mod command;
mod parse;
mod text;

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
