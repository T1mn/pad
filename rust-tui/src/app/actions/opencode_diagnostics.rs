use super::*;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

impl App {
    pub fn export_opencode_diagnostics(&mut self) -> bool {
        match export_diagnostics(&opencode_cli::opencode_command(&self.config)) {
            Ok(path) => {
                self.show_action_toast(
                    diagnostics_saved_title(self.locale),
                    &path.display().to_string(),
                );
                true
            }
            Err(err) => {
                self.show_action_toast(diagnostics_failed_title(self.locale), &err.to_string());
                false
            }
        }
    }
}

fn export_diagnostics(command: &OsString) -> io::Result<PathBuf> {
    let sections = [
        diagnostics_section(command, "version", &["--version"]),
        diagnostics_section(command, "db path", &["db", "path"]),
        diagnostics_section(command, "debug info", &["debug", "info"]),
        diagnostics_section(command, "debug paths", &["debug", "paths"]),
        diagnostics_section(command, "debug config", &["debug", "config"]),
        diagnostics_section(command, "providers list", &["providers", "list"]),
        diagnostics_section(command, "models --verbose", &["models", "--verbose"]),
        diagnostics_section(command, "agent list", &["agent", "list"]),
        diagnostics_section(command, "mcp list", &["mcp", "list"]),
    ];
    let path = diagnostics_path(
        crate::paths::opencode_diagnostics_dir().as_path(),
        current_unix_secs(),
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, format_report(&sections).as_bytes())?;
    Ok(path)
}

struct DiagnosticsSection {
    title: &'static str,
    body: String,
}

fn diagnostics_section(
    command: &OsString,
    title: &'static str,
    args: &[&str],
) -> DiagnosticsSection {
    let body = match run_opencode(command, args) {
        Ok(output) => output,
        Err(err) => format!("ERROR: {err}"),
    };
    DiagnosticsSection { title, body }
}

fn run_opencode(command: &OsString, args: &[&str]) -> io::Result<String> {
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(if stderr.is_empty() {
            format!("opencode {} exited with {}", args.join(" "), output.status)
        } else {
            stderr
        }));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        Err(io::Error::other(format!(
            "opencode {} returned empty output",
            args.join(" ")
        )))
    } else {
        Ok(stdout)
    }
}

fn format_report(sections: &[DiagnosticsSection]) -> String {
    let mut report = String::from("# OpenCode diagnostics\n");
    for section in sections {
        report.push_str("\n## ");
        report.push_str(section.title);
        report.push_str("\n\n");
        report.push_str(section.body.trim_end());
        report.push('\n');
    }
    report
}

fn diagnostics_path(dir: &Path, timestamp: u64) -> PathBuf {
    dir.join(format!("opencode-diagnostics-{timestamp}.txt"))
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

fn diagnostics_saved_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 诊断已导出"
    } else {
        "OpenCode Diagnostics Exported"
    }
}

fn diagnostics_failed_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "OpenCode 诊断失败"
    } else {
        "OpenCode Diagnostics Failed"
    }
}

#[cfg(test)]
#[path = "opencode_diagnostics_tests.rs"]
mod opencode_diagnostics_tests;
