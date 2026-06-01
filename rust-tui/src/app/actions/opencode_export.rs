use super::*;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

impl App {
    pub fn export_selected_opencode_thread(&mut self) -> bool {
        self.export_selected_opencode_thread_with_options(ExportMode::Raw)
    }

    pub fn export_sanitized_selected_opencode_thread(&mut self) -> bool {
        self.export_selected_opencode_thread_with_options(ExportMode::Sanitized)
    }

    fn export_selected_opencode_thread_with_options(&mut self, mode: ExportMode) -> bool {
        let Some(thread) = self.selected_preview_thread() else {
            self.show_action_toast(
                export_failed_title(self.locale),
                no_thread_message(self.locale),
            );
            return false;
        };
        if thread.agent_type != AgentType::OpenCode {
            self.show_action_toast(
                export_failed_title(self.locale),
                opencode_only_message(self.locale),
            );
            return false;
        }
        let Some(session_id) = thread
            .session_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
        else {
            self.show_action_toast(
                export_failed_title(self.locale),
                missing_session_message(self.locale),
            );
            return false;
        };

        match export_opencode_session(session_id, &self.opencode_export_command(), mode) {
            Ok(path) => {
                self.show_action_toast(
                    export_saved_title(self.locale, mode),
                    &path.display().to_string(),
                );
                true
            }
            Err(err) => {
                self.show_action_toast(export_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }

    fn opencode_export_command(&self) -> OsString {
        self.config
            .agents
            .iter()
            .find(|agent| agent.name == "opencode")
            .map(|agent| first_command_token(&agent.cmd))
            .filter(|cmd| !cmd.is_empty())
            .map(OsString::from)
            .unwrap_or_else(default_opencode_command)
    }
}

#[derive(Clone, Copy)]
enum ExportMode {
    Raw,
    Sanitized,
}

fn export_opencode_session(
    session_id: &str,
    command: &OsString,
    mode: ExportMode,
) -> io::Result<PathBuf> {
    let mut args = vec!["export", session_id];
    if matches!(mode, ExportMode::Sanitized) {
        args.push("--sanitize");
    }
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(if stderr.is_empty() {
            format!("opencode export exited with {}", output.status)
        } else {
            stderr
        }));
    }

    let body = String::from_utf8_lossy(&output.stdout);
    if body.trim().is_empty() {
        return Err(io::Error::other("opencode export returned empty output"));
    }

    let path = opencode_export_path(
        session_id,
        crate::paths::opencode_exports_dir().as_path(),
        mode,
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, body.as_bytes())?;
    Ok(path)
}

fn opencode_export_path(session_id: &str, dir: &Path, mode: ExportMode) -> PathBuf {
    let suffix = match mode {
        ExportMode::Raw => "json",
        ExportMode::Sanitized => "sanitized.json",
    };
    dir.join(format!("{}.{}", safe_filename(session_id), suffix))
}

fn safe_filename(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        "session".to_string()
    } else {
        out.chars().take(96).collect()
    }
}

fn first_command_token(command: &str) -> String {
    command
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn default_opencode_command() -> OsString {
    let home_bin = crate::paths::pad_home_dir()
        .parent()
        .map(|home| home.join(".opencode").join("bin").join("opencode"));
    if let Some(path) = home_bin.filter(|path| path.exists()) {
        path.into_os_string()
    } else {
        OsString::from("opencode")
    }
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

fn export_saved_title(locale: Locale, mode: ExportMode) -> &'static str {
    match (is_cjk_locale(locale), mode) {
        (true, ExportMode::Raw) => "OpenCode 已导出",
        (true, ExportMode::Sanitized) => "OpenCode 已脱敏导出",
        (false, ExportMode::Raw) => "OpenCode Exported",
        (false, ExportMode::Sanitized) => "OpenCode Sanitized Exported",
    }
}

fn export_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode 导出失败", "OpenCode Export Failed")
}

fn no_thread_message(locale: Locale) -> &'static str {
    localized(locale, "没有选中的线程", "No selected thread")
}

fn opencode_only_message(locale: Locale) -> &'static str {
    localized(locale, "只支持 OpenCode 会话", "Only OpenCode sessions")
}

fn missing_session_message(locale: Locale) -> &'static str {
    localized(
        locale,
        "选中的 OpenCode 线程缺少 session id",
        "Missing OpenCode session id",
    )
}

#[cfg(test)]
#[path = "opencode_export_tests.rs"]
mod opencode_export_tests;
