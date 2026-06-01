use super::*;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Command;

impl App {
    pub fn open_opencode_web_for_selected_thread(&mut self) -> bool {
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match open_opencode_web(&cwd, &opencode_cli::opencode_command(&self.config)) {
            Ok(()) => {
                self.show_action_toast(web_opened_title(self.locale), &cwd.display().to_string());
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(web_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn open_opencode_web(cwd: &PathBuf, command: &OsString) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(web_command(command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

fn web_command(command: &OsString) -> String {
    format!("{} web", shell_single_quote(&command.to_string_lossy()))
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

fn web_opened_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode Web 已打开", "OpenCode Web Opened")
}

fn web_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode Web 失败", "OpenCode Web Failed")
}

#[cfg(test)]
#[path = "opencode_web_tests.rs"]
mod opencode_web_tests;
