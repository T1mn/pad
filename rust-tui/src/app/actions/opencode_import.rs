use super::*;
use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;

impl App {
    pub fn import_opencode_thread_from_clipboard(&mut self) -> bool {
        let source = match crate::app::clipboard::read_text_from_clipboard() {
            Ok(text) => match normalize_import_source(&text) {
                Ok(source) => source,
                Err(message) => {
                    self.show_action_toast(import_failed_title(self.locale), message);
                    return false;
                }
            },
            Err(err) => {
                self.show_action_toast(import_failed_title(self.locale), &err.to_string());
                return false;
            }
        };

        match import_opencode_session(&source, &opencode_cli::opencode_command(&self.config)) {
            Ok(message) => {
                self.invalidate_sidebar_cache();
                self.sync_sidebar_selection();
                self.invalidate_preview();
                self.show_action_toast(import_saved_title(self.locale), &message);
                true
            }
            Err(err) => {
                self.show_action_toast(import_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn import_opencode_session(source: &str, command: &OsString) -> io::Result<String> {
    let output = Command::new(command).args(["import", source]).output()?;
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

fn normalize_import_source(text: &str) -> Result<String, &'static str> {
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

fn is_opencode_share_url(value: &str) -> bool {
    value.starts_with("https://") && value.contains("/s/")
}

fn is_json_path(value: &str) -> bool {
    value.ends_with(".json") || value.ends_with(".sanitized.json") || Path::new(value).exists()
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

fn import_saved_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 已导入"
    } else {
        "OpenCode Imported"
    }
}

fn import_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 导入失败"
    } else {
        "OpenCode Import Failed"
    }
}

#[cfg(test)]
#[path = "opencode_import_tests.rs"]
mod opencode_import_tests;
