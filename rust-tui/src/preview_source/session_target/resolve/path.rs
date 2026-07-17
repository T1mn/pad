use super::super::sources::{
    claude_transcript_path_for_session_id_from_thread, codex_transcript_path_for_session_id,
    find_matching_jsonl, gemini_transcript_path_for_session_id_from_thread,
    grok_transcript_path_for_session_id,
};
use crate::model::AgentType;
use crate::preview_source::PreviewRequest;
use std::path::PathBuf;
use std::time::Instant;

pub(super) fn resolve_transcript_path(
    original_request: &PreviewRequest,
    request: &PreviewRequest,
    claude_thread: Option<&crate::claude_history::ClaudeThreadRef>,
    gemini_thread: Option<&crate::gemini_history::GeminiThreadRef>,
) -> Option<PathBuf> {
    if let Some(candidate) = existing_request_transcript_path(request) {
        return Some(candidate);
    }

    let session_id = request.agent_session_id.as_deref()?;
    match request.agent_type {
        AgentType::Codex => codex_transcript_path(session_id),
        AgentType::Claude => claude_transcript_path(original_request, session_id, claude_thread),
        AgentType::Gemini => {
            gemini_transcript_path_for_session_id_from_thread(session_id, gemini_thread)
        }
        AgentType::Grok => grok_transcript_path_for_session_id(session_id),
        AgentType::OpenCode => opencode_transcript_path(session_id),
        _ => None,
    }
}

fn existing_request_transcript_path(request: &PreviewRequest) -> Option<PathBuf> {
    let candidate = PathBuf::from(request.transcript_path.as_ref()?);
    if request.agent_type == AgentType::Codex {
        return crate::codex_rollout::existing_rollout_path(&candidate);
    }
    candidate.exists().then_some(candidate)
}

fn codex_transcript_path(session_id: &str) -> Option<PathBuf> {
    codex_transcript_path_for_session_id(session_id).or_else(|| {
        find_matching_jsonl(&dirs::home_dir()?.join(".codex").join("sessions"), |name| {
            (name.ends_with(".jsonl") || name.ends_with(".jsonl.zst")) && name.contains(session_id)
        })
    })
}

fn claude_transcript_path(
    original_request: &PreviewRequest,
    session_id: &str,
    claude_thread: Option<&crate::claude_history::ClaudeThreadRef>,
) -> Option<PathBuf> {
    claude_transcript_path_for_session_id_from_thread(session_id, claude_thread)
        .or_else(|| claude_transcript_path_from_filesystem(original_request, session_id))
}

fn claude_transcript_path_from_filesystem(
    original_request: &PreviewRequest,
    session_id: &str,
) -> Option<PathBuf> {
    let started_at = Instant::now();
    let expected = format!("{}.jsonl", session_id);
    let path = find_matching_jsonl(&crate::paths::claude_projects_dir(), |name| {
        name == expected
    });
    if started_at.elapsed().as_millis() >= 15 {
        crate::log_debug!(
            "session_target.resolve: target={} agent=claude fallback=filesystem elapsed_ms={} session_id={} hit={}",
            original_request.target_key,
            started_at.elapsed().as_millis(),
            session_id,
            path.is_some()
        );
    }
    path
}

fn opencode_transcript_path(session_id: &str) -> Option<PathBuf> {
    crate::opencode_history::thread_for_id(session_id)
        .ok()
        .flatten()
        .map(|thread| thread.db_path)
}
