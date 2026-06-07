use crate::model::{AgentState, AgentType};
use crate::preview_source::PreviewRequest;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub(super) fn codex_thread_for_working_dir(
    working_dir: &str,
) -> Option<crate::codex_state::CodexThreadRef> {
    crate::codex_state::latest_thread_for_cwd(Path::new(working_dir))
        .ok()
        .flatten()
}

pub(super) fn opencode_thread_for_working_dir(
    working_dir: &str,
) -> Option<crate::opencode_history::OpenCodeThreadRef> {
    crate::opencode_history::threads_for_cwd(Path::new(working_dir))
        .ok()
        .and_then(|threads| threads.into_iter().next())
}

pub(super) fn codex_transcript_path_for_session_id(session_id: &str) -> Option<PathBuf> {
    crate::codex_state::thread_for_id(session_id)
        .ok()
        .flatten()
        .map(|thread| thread.rollout_path)
        .filter(|path| path.exists())
}

pub(super) fn claude_thread_for_session_id(
    session_id: &str,
) -> Option<crate::claude_history::ClaudeThreadRef> {
    crate::claude_history::thread_for_id(session_id)
        .ok()
        .flatten()
}

pub(super) fn claude_transcript_path_for_session_id_from_thread(
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

pub(super) fn gemini_thread_for_request(
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

pub(crate) fn resolved_session_id_for_request(
    request: &PreviewRequest,
    gemini_thread: Option<&crate::gemini_history::GeminiThreadRef>,
) -> Option<String> {
    if request.agent_type == AgentType::Gemini {
        return request
            .agent_session_id
            .clone()
            .or_else(|| gemini_thread.map(|thread| thread.session_id.clone()))
            .or_else(|| {
                request.transcript_path.as_deref().and_then(|path| {
                    super::super::gemini::extract_session_id_from_transcript(Path::new(path))
                })
            });
    }

    request
        .agent_session_id
        .clone()
        .or_else(|| {
            if request.agent_type == AgentType::Codex {
                codex_thread_for_working_dir(&request.working_dir).map(|thread| thread.thread_id)
            } else if request.agent_type == AgentType::OpenCode {
                opencode_thread_for_working_dir(&request.working_dir)
                    .map(|thread| thread.session_id)
            } else {
                None
            }
        })
        .or_else(|| {
            if request.transcript_path.is_some() {
                None
            } else if request.agent_type == AgentType::Codex && request.state == AgentState::Idle {
                request
                    .live_pane_id
                    .as_deref()
                    .and_then(crate::preview_source::codex::resolve_live_session_id)
            } else {
                None
            }
        })
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

pub(super) fn gemini_thread_for_transcript_path(
    transcript_path: &Path,
) -> Option<crate::gemini_history::GeminiThreadRef> {
    let threads = crate::gemini_history::all_threads().ok()?;
    threads
        .into_iter()
        .find(|thread| same_path(&thread.transcript_path, transcript_path))
}

pub(super) fn gemini_transcript_path_for_session_id_from_thread(
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

pub(super) fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub(crate) fn transcript_updated_at(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
}

pub(super) fn find_matching_jsonl<F>(root: &Path, matcher: F) -> Option<PathBuf>
where
    F: Fn(&str) -> bool,
{
    if !root.exists() {
        return None;
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let file_name = path.file_name()?.to_string_lossy();
            if matcher(&file_name) {
                return Some(path);
            }
        }
    }

    None
}
