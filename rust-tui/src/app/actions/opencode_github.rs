use super::*;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Command;

impl App {
    pub fn install_opencode_github_agent(&mut self) -> bool {
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match install_github_agent(&cwd, &opencode_cli::opencode_command(&self.config)) {
            Ok(()) => {
                self.show_action_toast(
                    github_started_title(self.locale),
                    &cwd.display().to_string(),
                );
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(github_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn install_github_agent(cwd: &PathBuf, command: &OsString) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(github_install_command(command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

fn github_install_command(command: &OsString) -> String {
    format!(
        "{} github install",
        shell_single_quote(&command.to_string_lossy())
    )
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

fn github_started_title(locale: Locale) -> &'static str {
    localized(
        locale,
        "OpenCode GitHub install 已启动",
        "OpenCode GitHub Install Started",
    )
}

fn github_failed_title(locale: Locale) -> &'static str {
    localized(
        locale,
        "OpenCode GitHub install 失败",
        "OpenCode GitHub Install Failed",
    )
}

#[cfg(test)]
#[path = "opencode_github_tests.rs"]
mod opencode_github_tests;
