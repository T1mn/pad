use std::path::{Path, PathBuf};

const SHARED_ROLLOUT_DIRS: &[&str] = &["sessions", "archived_sessions"];

pub(super) fn rollout_path_replacements(
    pad_codex_home: &Path,
    canonical_codex_home: &Path,
) -> Vec<(String, String)> {
    let mut replacements = Vec::new();
    for dir in SHARED_ROLLOUT_DIRS {
        let from = path_string(pad_codex_home.join(dir));
        let to = path_string(canonical_codex_home.join(dir));
        if from != to {
            replacements.push((from, to));
        }
    }
    replacements
}

fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().trim_end_matches('/').to_string()
}
