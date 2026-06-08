mod command;
mod prompt;
mod text;

use super::*;
use std::path::PathBuf;

impl App {
    pub fn run_opencode_prompt_from_clipboard(&mut self) -> bool {
        let prompt = match prompt_from_clipboard() {
            Ok(prompt) => prompt,
            Err(message) => {
                self.show_action_toast(text::run_failed_title(self.locale), &message);
                return false;
            }
        };

        let selected = self.selected_preview_thread();
        let cwd = selected
            .as_ref()
            .map(|thread| PathBuf::from(&thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let session_id = selected
            .as_ref()
            .filter(|thread| thread.agent_type == AgentType::OpenCode)
            .and_then(|thread| thread.session_id.as_deref())
            .map(str::trim)
            .filter(|session_id| !session_id.is_empty());

        match command::run_opencode_prompt(
            &prompt,
            session_id,
            &cwd,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(()) => {
                self.show_action_toast(
                    text::run_started_title(self.locale),
                    prompt::prompt_preview(&prompt),
                );
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(text::run_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn prompt_from_clipboard() -> Result<String, String> {
    let text = crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
    prompt::normalize_prompt(&text).map_err(str::to_string)
}

#[cfg(test)]
pub(in crate::app::actions) use command::run_command;
#[cfg(test)]
pub(in crate::app::actions) use prompt::{normalize_prompt, prompt_preview};

#[cfg(test)]
#[path = "opencode_run_tests.rs"]
mod opencode_run_tests;
