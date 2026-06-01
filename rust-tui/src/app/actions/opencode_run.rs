use super::*;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Command;

impl App {
    pub fn run_opencode_prompt_from_clipboard(&mut self) -> bool {
        let prompt = match crate::app::clipboard::read_text_from_clipboard() {
            Ok(text) => match normalize_prompt(&text) {
                Ok(prompt) => prompt,
                Err(message) => {
                    self.show_action_toast(run_failed_title(self.locale), message);
                    return false;
                }
            },
            Err(err) => {
                self.show_action_toast(run_failed_title(self.locale), &err.to_string());
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

        match run_opencode_prompt(
            &prompt,
            session_id,
            &cwd,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(()) => {
                self.show_action_toast(run_started_title(self.locale), prompt_preview(&prompt));
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(run_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn run_opencode_prompt(
    prompt: &str,
    session_id: Option<&str>,
    cwd: &PathBuf,
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

fn run_command(prompt: &str, session_id: Option<&str>, command: &OsString) -> String {
    let mut parts = vec![
        shell_single_quote(&command.to_string_lossy()),
        "run".to_string(),
    ];
    if let Some(session_id) = session_id {
        parts.push("--session".to_string());
        parts.push(shell_single_quote(session_id));
    }
    parts.push("--".to_string());
    parts.push(shell_single_quote(prompt));
    parts.join(" ")
}

fn normalize_prompt(text: &str) -> Result<String, &'static str> {
    let prompt = text.trim();
    if prompt.is_empty() {
        Err("Clipboard is empty")
    } else {
        Ok(prompt.to_string())
    }
}

fn prompt_preview(prompt: &str) -> &str {
    prompt
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(prompt)
}

fn shell_single_quote(value: &str) -> String {
    crate::codex_runtime::shell_single_quote(value)
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

fn localized(locale: Locale, zh: &'static str, en: &'static str) -> &'static str {
    if is_cjk_locale(locale) {
        zh
    } else {
        en
    }
}

fn run_started_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode run 已启动", "OpenCode Run Started")
}

fn run_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode run 失败", "OpenCode Run Failed")
}

#[cfg(test)]
#[path = "opencode_run_tests.rs"]
mod opencode_run_tests;
