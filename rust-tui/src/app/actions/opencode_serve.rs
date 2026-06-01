use super::*;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Command;

impl App {
    pub fn serve_opencode_for_selected_thread(&mut self) -> bool {
        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match serve_opencode(&cwd, &opencode_cli::opencode_command(&self.config)) {
            Ok(()) => {
                self.show_action_toast(
                    serve_started_title(self.locale),
                    &cwd.display().to_string(),
                );
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(serve_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn serve_opencode(cwd: &PathBuf, command: &OsString) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(serve_command(command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

fn serve_command(command: &OsString) -> String {
    format!(
        "{} serve --hostname 127.0.0.1 --port 0",
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

fn serve_started_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode serve 已启动", "OpenCode Serve Started")
}

fn serve_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode serve 失败", "OpenCode Serve Failed")
}

#[cfg(test)]
#[path = "opencode_serve_tests.rs"]
mod opencode_serve_tests;
