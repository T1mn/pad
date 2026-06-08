use super::path::same_path;
use crate::preview_source::PreviewRequest;
use std::path::{Path, PathBuf};

pub(in crate::preview_source::session_target) fn gemini_thread_for_request(
    request: &PreviewRequest,
) -> Option<crate::gemini_history::GeminiThreadRef> {
    if let Some(session_id) = request.agent_session_id.as_deref() {
        if let Some(thread) = gemini_thread_for_session_id(session_id) {
            return Some(thread);
        }
    }

    if let Some(path) = request.transcript_path.as_deref() {
        if let Some(thread) = gemini_thread_for_transcript_path(Path::new(path)) {
            return Some(thread);
        }
    }

    gemini_thread_for_working_dir(&request.working_dir)
}

pub(super) fn gemini_thread_for_session_id(
    session_id: &str,
) -> Option<crate::gemini_history::GeminiThreadRef> {
    crate::gemini_history::thread_for_id(session_id)
        .ok()
        .flatten()
}

pub(super) fn gemini_thread_for_working_dir(
    working_dir: &str,
) -> Option<crate::gemini_history::GeminiThreadRef> {
    let threads = crate::gemini_history::threads_for_cwd(Path::new(working_dir)).ok()?;
    if let Some(thread) = threads.iter().find(|thread| thread.kind == "main").cloned() {
        return Some(thread);
    }
    threads.into_iter().next()
}

fn gemini_thread_for_transcript_path(
    transcript_path: &Path,
) -> Option<crate::gemini_history::GeminiThreadRef> {
    let threads = crate::gemini_history::all_threads().ok()?;
    threads
        .into_iter()
        .find(|thread| same_path(&thread.transcript_path, transcript_path))
}

pub(in crate::preview_source::session_target) fn gemini_transcript_path_for_session_id_from_thread(
    session_id: &str,
    gemini_thread: Option<&crate::gemini_history::GeminiThreadRef>,
) -> Option<PathBuf> {
    let thread = gemini_thread?;
    if thread.session_id != session_id {
        return None;
    }
    let transcript_path = thread.transcript_path.clone();
    transcript_path.exists().then_some(transcript_path)
}
