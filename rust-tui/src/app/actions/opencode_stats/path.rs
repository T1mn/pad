use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::app::actions) fn opencode_stats_path(
    project: &str,
    dir: &Path,
    timestamp: u64,
) -> PathBuf {
    let stem = super::super::opencode_cli::safe_filename(project)
        .trim_start_matches('_')
        .to_string();
    dir.join(format!("{}-{}.txt", stem, timestamp))
}

pub(super) fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
