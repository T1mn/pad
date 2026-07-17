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
