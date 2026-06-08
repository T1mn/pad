mod export;
mod mode;
mod path;
mod text;

use super::{opencode_cli, App};
use crate::model::AgentType;
use mode::ExportMode;
use text::{
    export_failed_title, export_saved_title, missing_session_message, no_thread_message,
    opencode_only_message,
};

impl App {
    pub fn export_selected_opencode_thread(&mut self) -> bool {
        self.export_selected_opencode_thread_with_options(ExportMode::Raw)
    }

    pub fn export_sanitized_selected_opencode_thread(&mut self) -> bool {
        self.export_selected_opencode_thread_with_options(ExportMode::Sanitized)
    }

    fn export_selected_opencode_thread_with_options(&mut self, mode: ExportMode) -> bool {
        let Some(thread) = self.selected_preview_thread() else {
            self.show_action_toast(
                export_failed_title(self.locale),
                no_thread_message(self.locale),
            );
            return false;
        };
        if thread.agent_type != AgentType::OpenCode {
            self.show_action_toast(
                export_failed_title(self.locale),
                opencode_only_message(self.locale),
            );
            return false;
        }
        let Some(session_id) = thread
            .session_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
        else {
            self.show_action_toast(
                export_failed_title(self.locale),
                missing_session_message(self.locale),
            );
            return false;
        };

        match export::export_opencode_session(
            session_id,
            &opencode_cli::opencode_command(&self.config),
            mode,
        ) {
            Ok(path) => {
                self.show_action_toast(
                    export_saved_title(self.locale, mode),
                    &path.display().to_string(),
                );
                true
            }
            Err(err) => {
                self.show_action_toast(export_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

#[cfg(test)]
use path::opencode_export_path;

#[cfg(test)]
#[path = "opencode_export_tests.rs"]
mod opencode_export_tests;
