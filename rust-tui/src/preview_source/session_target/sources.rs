mod claude {
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
}
mod codex;
mod gemini;
mod grok {
    use std::path::PathBuf;

    pub(in crate::preview_source::session_target) fn grok_transcript_path_for_session_id(
        session_id: &str,
    ) -> Option<PathBuf> {
        crate::grok_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .map(|thread| thread.transcript_path)
    }
}
mod opencode {
    use std::path::Path;

    pub(super) fn opencode_thread_for_working_dir(
        working_dir: &str,
        require_unique: bool,
    ) -> Option<crate::opencode_history::OpenCodeThreadRef> {
        let threads = crate::opencode_history::threads_for_cwd(Path::new(working_dir)).ok()?;
        super::path::select_cwd_candidate(threads, require_unique)
    }
}
mod path;
mod resolved;

pub(super) use claude::{
    claude_thread_for_session_id, claude_transcript_path_for_session_id_from_thread,
};
pub(super) use codex::codex_transcript_path_for_session_id;
pub(super) use gemini::{
    gemini_thread_for_request, gemini_transcript_path_for_session_id_from_thread,
};
pub(super) use grok::grok_transcript_path_for_session_id;
pub(super) use path::find_matching_jsonl;
pub(crate) use path::transcript_updated_at;
#[cfg(test)]
pub(crate) use resolved::resolved_session_id_for_request;
#[cfg(not(test))]
pub(super) use resolved::resolved_session_id_for_request;

#[cfg(test)]
mod tests;
