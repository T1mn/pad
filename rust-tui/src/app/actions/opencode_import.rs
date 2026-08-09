mod command {
    use std::io;

    pub(super) fn import_opencode_session(source: &str, command: &str) -> io::Result<String> {
        let output = super::super::opencode_cli::run_with_args(command, &["import", source], None)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(io::Error::other(if stderr.is_empty() {
                format!("opencode import exited with {}", output.status)
            } else {
                stderr
            }));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(if stdout.is_empty() {
            source.to_string()
        } else {
            stdout
        })
    }
}
mod source {
    use std::path::Path;

    use super::super::helpers::trim_wrapping_quotes;

    pub(in crate::app::actions) fn normalize_import_source(
        text: &str,
    ) -> Result<String, &'static str> {
        let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
        let Some(first) = lines.next() else {
            return Err("Clipboard is empty");
        };
        if lines.next().is_some() {
            return Err("Clipboard must contain one JSON path or OpenCode share URL");
        }

        let source = trim_wrapping_quotes(first);
        if is_opencode_share_url(source) || is_json_path(source) {
            Ok(source.to_string())
        } else {
            Err("Clipboard must contain a JSON path or OpenCode share URL")
        }
    }

    fn is_opencode_share_url(value: &str) -> bool {
        value.starts_with("https://") && value.contains("/s/")
    }

    fn is_json_path(value: &str) -> bool {
        value.ends_with(".json") || value.ends_with(".sanitized.json") || Path::new(value).exists()
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn import_saved_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode 已导入", "OpenCode Imported")
    }

    pub(super) fn import_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode 导入失败", "OpenCode Import Failed")
    }
}

use super::*;

impl App {
    pub fn import_opencode_thread_from_clipboard(&mut self) -> bool {
        let source = match source_from_clipboard() {
            Ok(source) => source,
            Err(message) => {
                self.show_action_toast(text::import_failed_title(self.locale), &message);
                return false;
            }
        };

        match command::import_opencode_session(
            &source,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(message) => {
                self.invalidate_sidebar_cache();
                self.sync_sidebar_selection();
                self.invalidate_preview();
                self.show_action_toast(text::import_saved_title(self.locale), &message);
                true
            }
            Err(err) => {
                self.show_action_toast(text::import_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn source_from_clipboard() -> Result<String, String> {
    let text = crate::app::clipboard::read_text_from_clipboard().map_err(|err| err.to_string())?;
    source::normalize_import_source(&text).map_err(str::to_string)
}

#[cfg(test)]
pub(in crate::app::actions) use super::helpers::trim_wrapping_quotes;
#[cfg(test)]
pub(in crate::app::actions) use source::normalize_import_source;
