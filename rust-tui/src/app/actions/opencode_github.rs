mod command {
    pub(in crate::app::actions) fn github_install_command(command: &str) -> String {
        super::super::opencode_cli::command_with_args(command, ["github", "install"])
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn github_started_title(locale: Locale) -> &'static str {
        localized(
            locale,
            "OpenCode GitHub install 已启动",
            "OpenCode GitHub Install Started",
        )
    }

    pub(super) fn github_failed_title(locale: Locale) -> &'static str {
        localized(
            locale,
            "OpenCode GitHub install 失败",
            "OpenCode GitHub Install Failed",
        )
    }
}

use super::*;
use std::path::PathBuf;

impl App {
    pub fn install_opencode_github_agent(&mut self) -> bool {
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let command =
            command::github_install_command(&opencode_cli::opencode_command(&self.config));
        match self.launch_native_agent_action(
            "OpenCode GitHub Install",
            &command,
            AgentType::OpenCode,
            cwd.clone(),
        ) {
            Ok(_) => {
                self.show_action_toast(
                    text::github_started_title(self.locale),
                    &cwd.display().to_string(),
                );
                true
            }
            Err(err) => {
                self.show_action_toast(text::github_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

#[cfg(test)]
pub(in crate::app::actions) use command::github_install_command;
