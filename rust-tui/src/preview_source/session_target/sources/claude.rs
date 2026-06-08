use std::path::PathBuf;

pub(in crate::preview_source::session_target) fn claude_thread_for_session_id(
    session_id: &str,
) -> Option<crate::claude_history::ClaudeThreadRef> {
    crate::claude_history::thread_for_id(session_id)
        .ok()
        .flatten()
}

pub(in crate::preview_source::session_target) fn claude_transcript_path_for_session_id_from_thread(
    session_id: &str,
    claude_thread: Option<&crate::claude_history::ClaudeThreadRef>,
) -> Option<PathBuf> {
    let thread = claude_thread?;
    if thread.session_id != session_id {
        return None;
    }
    let transcript_path = thread.transcript_path.clone();
    transcript_path.exists().then_some(transcript_path)
}
