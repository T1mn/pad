use crate::session_continuity::model::ContinuityDiagnosticEvent;
use crate::session_continuity::storage::with_continuity_lock;
use std::fs::{self, OpenOptions};
use std::io::Write;

pub(in crate::session_continuity) fn append_diagnostic(event: &ContinuityDiagnosticEvent) {
    let _ = with_continuity_lock(|| append_diagnostic_unlocked(event));
}

fn append_diagnostic_unlocked(event: &ContinuityDiagnosticEvent) {
    let path = crate::paths::session_continuity_log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            if let Ok(line) = serde_json::to_string(event) {
                let _ = writeln!(file, "{}", line);
            }
        }
        Err(err) => {
            crate::log_debug!(
                "session_continuity: failed to append diagnostic err={}",
                err
            );
        }
    }
}
