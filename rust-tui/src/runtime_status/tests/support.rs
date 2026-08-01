use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn temp_status_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "pad-status-guard-{}-{}.json",
        std::process::id(),
        crate::time::unix_now_nanos()
    ))
}

pub(super) fn remove_status_artifacts(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(super::super::status_lock_path(path));
}
