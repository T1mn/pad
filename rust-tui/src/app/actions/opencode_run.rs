mod command {
    pub(in crate::app::actions) fn run_command(
        prompt: &str,
        session_id: Option<&str>,
        command: &str,
    ) -> String {
        let mut command_line = command.trim().to_string();
        command_line.push_str(" run");
        if let Some(session_id) = session_id {
            command_line.push_str(" --session ");
            command_line.push_str(&crate::codex_runtime::shell_single_quote(session_id));
        }
        command_line.push_str(" -- ");
        command_line.push_str(&single_line_shell_value(prompt));
        command_line
    }

    fn single_line_shell_value(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len());
        for character in value.chars() {
            match character {
                '\0' => escaped.push('\0'),
                '\\' => escaped.push_str("\\\\"),
                character if character.is_control() => {
                    let mut encoded = [0; 4];
                    for byte in character.encode_utf8(&mut encoded).bytes() {
                        push_octal_byte(&mut escaped, byte);
                    }
                }
                character => escaped.push(character),
            }
        }
        format!(
            "\"$(printf '%b' {})\"",
            crate::codex_runtime::shell_single_quote(&escaped)
        )
    }

    fn push_octal_byte(output: &mut String, byte: u8) {
        output.push_str("\\0");
        output.push(char::from(b'0' + ((byte >> 6) & 7)));
        output.push(char::from(b'0' + ((byte >> 3) & 7)));
        output.push(char::from(b'0' + (byte & 7)));
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

        let command = command::run_command(
            &prompt,
            session_id,
            &opencode_cli::opencode_command(&self.config),
        );
        match self.launch_native_agent_action("OpenCode Run", &command, AgentType::OpenCode, cwd) {
            Ok(_) => {
                self.show_action_toast(
                    text::run_started_title(self.locale),
                    prompt::prompt_preview(&prompt),
                );
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
