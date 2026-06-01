use super::*;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

impl App {
    pub fn export_selected_opencode_thread(&mut self) -> bool {
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

        match export_opencode_session(session_id, &self.opencode_export_command()) {
            Ok(path) => {
                self.show_action_toast(
                    export_saved_title(self.locale),
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

fn export_opencode_session(session_id: &str, command: &OsString) -> io::Result<PathBuf> {
    let output = Command::new(command)
        .args(["export", session_id])
        .output()?;
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

    let path = opencode_export_path(session_id, crate::paths::opencode_exports_dir().as_path());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, body.as_bytes())?;
    Ok(path)
}

fn opencode_export_path(session_id: &str, dir: &Path) -> PathBuf {
    dir.join(format!("{}.json", safe_filename(session_id)))
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

fn export_saved_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 已导出"
    } else {
        "OpenCode Exported"
    }
}

fn export_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 导出失败"
    } else {
        "OpenCode Export Failed"
    }
}

fn no_thread_message(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "没有选中的线程"
    } else {
        "No selected thread"
    }
}

fn opencode_only_message(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "只支持 OpenCode 会话"
    } else {
        "Only OpenCode sessions can be exported"
    }
}

fn missing_session_message(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "选中的 OpenCode 线程缺少 session id"
    } else {
        "Selected OpenCode thread is missing session id"
    }
}

#[cfg(test)]
mod tests {
    use super::{first_command_token, opencode_export_path, safe_filename};
    use std::path::Path;

    #[test]
    fn opencode_export_path_sanitizes_session_id() {
        assert_eq!(
            opencode_export_path("ses/../abc def", Path::new("/tmp/out")),
            Path::new("/tmp/out/ses_abc_def.json")
        );
    }

    #[test]
    fn opencode_command_uses_first_configured_token() {
        assert_eq!(
            first_command_token("/opt/bin/opencode --pure"),
            "/opt/bin/opencode"
        );
        assert_eq!(safe_filename("***"), "session");
    }
}
