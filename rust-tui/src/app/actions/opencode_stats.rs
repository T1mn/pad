mod export;
mod path;
mod text;

use super::*;

impl App {
    pub fn export_selected_opencode_stats(&mut self) -> bool {
        let Some(thread) = self.selected_preview_thread() else {
            self.show_action_toast(
                text::stats_failed_title(self.locale),
                text::no_thread_message(self.locale),
            );
            return false;
        };
        if thread.agent_type != AgentType::OpenCode {
            self.show_action_toast(
                text::stats_failed_title(self.locale),
                text::opencode_only_message(self.locale),
            );
            return false;
        }

        match export::export_opencode_stats(
            &thread.working_dir,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(path) => {
                self.show_action_toast(
                    text::stats_saved_title(self.locale),
                    &path.display().to_string(),
                );
                true
            }
            Err(err) => {
                self.show_action_toast(text::stats_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

#[cfg(test)]
pub(in crate::app::actions) use path::opencode_stats_path;

#[cfg(test)]
#[path = "opencode_stats_tests.rs"]
mod opencode_stats_tests;
