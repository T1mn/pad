mod command;
mod text;
mod url;

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

#[cfg(test)]
#[path = "opencode_attach_tests.rs"]
mod opencode_attach_tests;
