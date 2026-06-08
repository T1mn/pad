use super::collect::DiagnosticsSection;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::app::actions) fn format_report(sections: &[DiagnosticsSection]) -> String {
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

pub(in crate::app::actions) fn diagnostics_path(dir: &Path, timestamp: u64) -> PathBuf {
    dir.join(format!("opencode-diagnostics-{timestamp}.txt"))
}

pub(super) fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
