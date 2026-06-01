use super::*;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Command;

impl App {
    pub fn install_opencode_plugin_from_clipboard(&mut self) -> bool {
        let module = match crate::app::clipboard::read_text_from_clipboard() {
            Ok(text) => match normalize_plugin_module(&text) {
                Ok(module) => module,
                Err(message) => {
                    self.show_action_toast(plugin_failed_title(self.locale), message);
                    return false;
                }
            },
            Err(err) => {
                self.show_action_toast(plugin_failed_title(self.locale), &err.to_string());
                return false;
            }
        };

        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match install_opencode_plugin(&module, &cwd, &opencode_cli::opencode_command(&self.config))
        {
            Ok(()) => {
                self.show_action_toast(plugin_started_title(self.locale), &module);
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(plugin_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn install_opencode_plugin(module: &str, cwd: &PathBuf, command: &OsString) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(plugin_command(module, command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

fn plugin_command(module: &str, command: &OsString) -> String {
    format!(
        "{} plugin {}",
        shell_single_quote(&command.to_string_lossy()),
        shell_single_quote(module)
    )
}

fn normalize_plugin_module(text: &str) -> Result<String, &'static str> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first) = lines.next() else {
        return Err("Clipboard is empty");
    };
    if lines.next().is_some() {
        return Err("Clipboard must contain one npm module name");
    }
    let module = trim_wrapping_quotes(first);
    if is_safe_module_name(module) {
        Ok(module.to_string())
    } else {
        Err("Clipboard must contain an npm module name, not CLI flags or whitespace")
    }
}

fn is_safe_module_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.contains(char::is_whitespace)
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '@' | '/' | '-' | '_' | '.' | '~'))
}

fn trim_wrapping_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
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

fn plugin_started_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode plugin 已启动", "OpenCode Plugin Started")
}

fn plugin_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode plugin 失败", "OpenCode Plugin Failed")
}

#[cfg(test)]
#[path = "opencode_plugin_tests.rs"]
mod opencode_plugin_tests;
