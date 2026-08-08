mod collect;
mod report {
    use super::collect::DiagnosticsSection;
    use std::io;
    use std::path::{Path, PathBuf};

    pub(in crate::app::actions) fn format_report(sections: &[DiagnosticsSection]) -> String {
        let mut report = String::from("# OpenCode diagnostics\n");
        for section in sections {
            report.push_str("\n## ");
            report.push_str(section.title);
            report.push_str("\n\n");
            report.push_str(redact_sensitive_text(section.body.trim_end()).as_str());
            report.push('\n');
        }
        report
    }

    pub(super) fn write_private_report(path: &Path, body: &str) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .mode(0o600)
                .open(path)?;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
            file.write_all(body.as_bytes())
        }
        #[cfg(not(unix))]
        {
            std::fs::write(path, body)
        }
    }

    fn redact_sensitive_text(body: &str) -> String {
        body.lines()
            .map(redact_sensitive_line)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn redact_sensitive_line(line: &str) -> String {
        if let Some(separator) = line.find([':', '=']) {
            let key = line[..separator]
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            let sensitive = [
                "apikey",
                "authorization",
                "credential",
                "password",
                "secret",
            ]
            .iter()
            .any(|sensitive| key.contains(sensitive))
                || key == "token"
                || key.ends_with("token");
            if sensitive {
                let trailing_comma = line.trim_end().ends_with(',');
                return format!(
                    "{}{} [REDACTED]{}",
                    &line[..separator],
                    &line[separator..=separator],
                    if trailing_comma { "," } else { "" }
                );
            }
        }
        redact_token_prefixes(line)
    }

    fn redact_token_prefixes(line: &str) -> String {
        const PREFIXES: &[&str] = &["sk-", "xai-", "anthropic-", "ghp_", "github_pat_"];
        let mut output = line.to_string();
        for prefix in PREFIXES {
            while let Some(start) = output.find(prefix) {
                let end = output[start..]
                    .find(|ch: char| ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | ';'))
                    .map(|offset| start + offset)
                    .unwrap_or(output.len());
                output.replace_range(start..end, "[REDACTED]");
            }
        }
        output
    }

    pub(in crate::app::actions) fn diagnostics_path(dir: &Path, timestamp: u64) -> PathBuf {
        dir.join(format!("opencode-diagnostics-{timestamp}.txt"))
    }

    pub(super) fn current_unix_secs() -> u64 {
        crate::time::unix_now_secs()
    }
}
mod text {
    use super::super::helpers::localized;
    use crate::i18n::Locale;

    pub(super) fn diagnostics_saved_title(locale: Locale) -> &'static str {
        localized(
            locale,
            "OpenCode 诊断已导出",
            "OpenCode Diagnostics Exported",
        )
    }

    pub(super) fn diagnostics_failed_title(locale: Locale) -> &'static str {
        localized(locale, "OpenCode 诊断失败", "OpenCode Diagnostics Failed")
    }
}

use super::*;
use std::io;
use std::path::PathBuf;

impl App {
    pub fn export_opencode_diagnostics(&mut self) -> bool {
        match export_diagnostics(&opencode_cli::opencode_command(&self.config)) {
            Ok(path) => {
                self.show_action_toast(
                    text::diagnostics_saved_title(self.locale),
                    &path.display().to_string(),
                );
                true
            }
            Err(err) => {
                self.show_action_toast(
                    text::diagnostics_failed_title(self.locale),
                    &err.to_string(),
                );
                false
            }
        }
    }
}

fn export_diagnostics(command: &str) -> io::Result<PathBuf> {
    let sections = collect::collect_diagnostics_sections(command);
    let path = report::diagnostics_path(
        crate::paths::opencode_diagnostics_dir().as_path(),
        report::current_unix_secs(),
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    report::write_private_report(&path, &report::format_report(&sections))?;
    Ok(path)
}

#[cfg(test)]
pub(in crate::app::actions) use collect::DiagnosticsSection;
#[cfg(test)]
pub(in crate::app::actions) use report::{diagnostics_path, format_report};

#[cfg(test)]
#[path = "opencode_diagnostics_tests.rs"]
mod opencode_diagnostics_tests;
