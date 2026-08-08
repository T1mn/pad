mod command {
    use std::ffi::OsString;
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub(super) fn run_opencode_prompt(
        prompt: &str,
        session_id: Option<&str>,
        cwd: &Path,
        command: &OsString,
    ) -> io::Result<()> {
        let status = Command::new("tmux")
            .args(["new-window", "-c"])
            .arg(cwd)
            .arg(run_command(prompt, session_id, command))
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "tmux new-window exited with {status}"
            )))
        }
    }

    pub(in crate::app::actions) fn run_command(
        prompt: &str,
        session_id: Option<&str>,
        command: &OsString,
    ) -> String {
        let mut command_line = crate::codex_runtime::shell_single_quote(&command.to_string_lossy());
        command_line.push_str(" run");
        if let Some(session_id) = session_id {
            command_line.push_str(" --session ");
            command_line.push_str(&crate::codex_runtime::shell_single_quote(session_id));
        }
        command_line.push_str(" -- ");
        command_line.push_str(&crate::codex_runtime::shell_single_quote(prompt));
        command_line
    }
}
mod prompt {
    pub(in crate::app::actions) fn normalize_prompt(text: &str) -> Result<String, &'static str> {
        let prompt = text.trim();
        if prompt.is_empty() {
            Err("Clipboard is empty")
        } else {
            Ok(prompt.to_string())
        }
    }

    pub(in crate::app::actions) fn prompt_preview(prompt: &str) -> &str {
        prompt
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or(prompt)
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn run_started_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode run 已启动", "OpenCode Run Started")
    }

    pub(super) fn run_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode run 失败", "OpenCode Run Failed")
    }
}

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
