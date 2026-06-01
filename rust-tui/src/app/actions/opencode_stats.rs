use super::*;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

impl App {
    pub fn export_selected_opencode_stats(&mut self) -> bool {
        let Some(thread) = self.selected_preview_thread() else {
            self.show_action_toast(
                stats_failed_title(self.locale),
                no_thread_message(self.locale),
            );
            return false;
        };
        if thread.agent_type != AgentType::OpenCode {
            self.show_action_toast(
                stats_failed_title(self.locale),
                opencode_only_message(self.locale),
            );
            return false;
        }

        match export_opencode_stats(
            &thread.working_dir,
            &opencode_cli::opencode_command(&self.config),
        ) {
            Ok(path) => {
                self.show_action_toast(stats_saved_title(self.locale), &path.display().to_string());
                true
            }
            Err(err) => {
                self.show_action_toast(stats_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn export_opencode_stats(project: &str, command: &OsString) -> io::Result<PathBuf> {
    let output = Command::new(command)
        .args([
            "stats",
            "--project",
            project,
            "--models",
            "10",
            "--tools",
            "10",
        ])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(if stderr.is_empty() {
            format!("opencode stats exited with {}", output.status)
        } else {
            stderr
        }));
    }

    let body = String::from_utf8_lossy(&output.stdout);
    if body.trim().is_empty() {
        return Err(io::Error::other("opencode stats returned empty output"));
    }

    let path = opencode_stats_path(
        project,
        crate::paths::opencode_stats_dir().as_path(),
        current_unix_secs(),
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, body.as_bytes())?;
    Ok(path)
}

fn opencode_stats_path(project: &str, dir: &Path, timestamp: u64) -> PathBuf {
    let stem = opencode_cli::safe_filename(project)
        .trim_start_matches('_')
        .to_string();
    dir.join(format!("{}-{}.txt", stem, timestamp))
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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

fn stats_saved_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode stats 已导出", "OpenCode Stats Exported")
}

fn stats_failed_title(locale: Locale) -> &'static str {
    localized(locale, "OpenCode stats 失败", "OpenCode Stats Failed")
}

fn no_thread_message(locale: Locale) -> &'static str {
    localized(locale, "没有选中的线程", "No selected thread")
}

fn opencode_only_message(locale: Locale) -> &'static str {
    localized(locale, "只支持 OpenCode 会话", "Only OpenCode sessions")
}

#[cfg(test)]
#[path = "opencode_stats_tests.rs"]
mod opencode_stats_tests;
