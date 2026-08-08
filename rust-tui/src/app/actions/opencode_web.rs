mod command {
    pub(in crate::app::actions) fn web_command(command: &str) -> String {
        super::super::opencode_cli::command_with_args(command, ["web"])
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn web_opened_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode Web 已打开", "OpenCode Web Opened")
    }

    pub(super) fn web_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode Web 失败", "OpenCode Web Failed")
    }
}

use super::*;
use std::path::PathBuf;

impl App {
    pub fn open_opencode_web_for_selected_thread(&mut self) -> bool {
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let command = command::web_command(&opencode_cli::opencode_command(&self.config));
        match self.launch_native_agent_action(
            "OpenCode Web",
            &command,
            AgentType::OpenCode,
            cwd.clone(),
        ) {
            Ok(_) => {
                self.show_action_toast(
                    text::web_opened_title(self.locale),
                    &cwd.display().to_string(),
                );
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
