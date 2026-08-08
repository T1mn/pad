mod command {
    pub(in crate::app::actions) fn serve_command(command: &str) -> String {
        super::super::opencode_cli::command_with_args(
            command,
            ["serve", "--hostname", "127.0.0.1", "--port", "0"],
        )
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn serve_started_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode serve 已启动", "OpenCode Serve Started")
    }

    pub(super) fn serve_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode serve 失败", "OpenCode Serve Failed")
    }
}

use super::*;
use std::path::PathBuf;

impl App {
    pub fn serve_opencode_for_selected_thread(&mut self) -> bool {
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let command = command::serve_command(&opencode_cli::opencode_command(&self.config));
        match self.launch_native_agent_action(
            "OpenCode Serve",
            &command,
            AgentType::OpenCode,
            cwd.clone(),
        ) {
            Ok(_) => {
                self.show_action_toast(
                    text::serve_started_title(self.locale),
                    &cwd.display().to_string(),
                );
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
