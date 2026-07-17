use std::path::{Path, PathBuf};

pub(super) fn codex_thread_for_working_dir(
    working_dir: &str,
    require_unique: bool,
) -> Option<crate::codex_state::CodexThreadRef> {
    if !require_unique {
        return crate::codex_state::latest_thread_for_cwd(Path::new(working_dir))
            .ok()
            .flatten();
    }
    let threads = crate::codex_state::threads_for_cwd(Path::new(working_dir)).ok()?;
    super::path::select_cwd_candidate(threads, require_unique)
}

pub(in crate::preview_source::session_target) fn codex_transcript_path_for_session_id(
    session_id: &str,
) -> Option<PathBuf> {
    crate::codex_state::thread_for_id(session_id)
        .ok()
        .flatten()
        .map(|thread| thread.rollout_path)
        .and_then(|path| crate::codex_rollout::existing_rollout_path(&path))
}

#[cfg(test)]
#[path = "codex_tests.rs"]
mod tests;
