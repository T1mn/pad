use std::path::{Path, PathBuf};

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
    crate::time::unix_now_secs()
}
