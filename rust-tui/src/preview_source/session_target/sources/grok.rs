use std::path::PathBuf;

pub(in crate::preview_source::session_target) fn grok_transcript_path_for_session_id(
    session_id: &str,
) -> Option<PathBuf> {
    crate::grok_history::thread_for_id(session_id)
        .ok()
        .flatten()
        .map(|thread| thread.transcript_path)
}
