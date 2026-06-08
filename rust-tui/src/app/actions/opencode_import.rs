mod command;
mod source;
mod text;

use super::*;

impl App {
    pub fn import_opencode_thread_from_clipboard(&mut self) -> bool {
        let source = match source_from_clipboard() {
            Ok(source) => source,
            Err(message) => {
                self.show_action_toast(text::import_failed_title(self.locale), &message);
                return false;
            }
        };

        match command::import_opencode_session(
            &source,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(message) => {
                self.invalidate_sidebar_cache();
                self.sync_sidebar_selection();
                self.invalidate_preview();
                self.show_action_toast(text::import_saved_title(self.locale), &message);
                true
            }
            Err(err) => {
                self.show_action_toast(text::import_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn source_from_clipboard() -> Result<String, String> {
    let text = crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
    source::normalize_import_source(&text).map_err(str::to_string)
}

#[cfg(test)]
pub(in crate::app::actions) use source::{normalize_import_source, trim_wrapping_quotes};

#[cfg(test)]
#[path = "opencode_import_tests.rs"]
mod opencode_import_tests;
