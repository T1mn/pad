mod command;
mod module;
mod text;

use super::*;
use std::path::PathBuf;

impl App {
    pub fn install_opencode_plugin_from_clipboard(&mut self) -> bool {
        let module = match module_from_clipboard() {
            Ok(module) => module,
            Err(message) => {
                self.show_action_toast(text::plugin_failed_title(self.locale), &message);
                return false;
            }
        };

        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match command::install_opencode_plugin(
            &module,
            &cwd,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(()) => {
                self.show_action_toast(text::plugin_started_title(self.locale), &module);
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(text::plugin_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn module_from_clipboard() -> Result<String, String> {
    let text = crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
    module::normalize_plugin_module(&text).map_err(str::to_string)
}

#[cfg(test)]
pub(in crate::app::actions) use command::plugin_command;
#[cfg(test)]
pub(in crate::app::actions) use module::normalize_plugin_module;

#[cfg(test)]
#[path = "opencode_plugin_tests.rs"]
mod opencode_plugin_tests;
