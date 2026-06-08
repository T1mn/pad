use std::path::{Path, PathBuf};

pub(super) fn codex_thread_for_working_dir(
    working_dir: &str,
) -> Option<crate::codex_state::CodexThreadRef> {
    crate::codex_state::latest_thread_for_cwd(Path::new(working_dir))
        .ok()
        .flatten()
}

pub(in crate::preview_source::session_target) fn codex_transcript_path_for_session_id(
    session_id: &str,
) -> Option<PathBuf> {
    crate::codex_state::thread_for_id(session_id)
        .ok()
        .flatten()
        .map(|thread| thread.rollout_path)
        .filter(|path| path.exists())
}
