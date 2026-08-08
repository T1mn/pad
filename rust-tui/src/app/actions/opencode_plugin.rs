mod command {
    use std::ffi::OsString;
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub(super) fn install_opencode_plugin(
        module: &str,
        cwd: &Path,
        command: &OsString,
    ) -> io::Result<()> {
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

    pub(in crate::app::actions) fn plugin_command(module: &str, command: &OsString) -> String {
        format!(
            "{} plugin {}",
            crate::codex_runtime::shell_single_quote(&command.to_string_lossy()),
            crate::codex_runtime::shell_single_quote(module)
        )
    }
}
mod module {
    use super::super::helpers::trim_wrapping_quotes;

    pub(in crate::app::actions) fn normalize_plugin_module(
        text: &str,
    ) -> Result<String, &'static str> {
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
            && value.chars().all(|ch| {
                ch.is_ascii_alphanumeric() || matches!(ch, '@' | '/' | '-' | '_' | '.' | '~')
            })
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn plugin_started_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode plugin 已启动", "OpenCode Plugin Started")
    }

    pub(super) fn plugin_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode plugin 失败", "OpenCode Plugin Failed")
    }
}

use super::*;
use std::path::PathBuf;

impl App {
    pub fn install_opencode_plugin_from_clipboard(&mut self) -> bool {
        let module = match module_from_clipboard() {
            Ok(module) => module,
            Err(message) => {
                self.show_action_toast(text::plugin_failed_title(self.locale), &message);
                return false;
            }
        };

        let cwd = self
            .selected_preview_thread()
            .map(|thread| PathBuf::from(thread.working_dir))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        match command::install_opencode_plugin(
            &module,
            &cwd,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(()) => {
                self.show_action_toast(text::plugin_started_title(self.locale), &module);
                self.schedule_delayed_scan(800);
                true
            }
            Err(err) => {
                self.show_action_toast(text::plugin_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn module_from_clipboard() -> Result<String, String> {
    let text = crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
    module::normalize_plugin_module(&text).map_err(str::to_string)
}

#[cfg(test)]
pub(in crate::app::actions) use command::plugin_command;
#[cfg(test)]
pub(in crate::app::actions) use module::normalize_plugin_module;
