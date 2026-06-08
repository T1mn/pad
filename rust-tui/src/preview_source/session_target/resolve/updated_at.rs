use super::super::sources::transcript_updated_at;
use crate::model::AgentType;
use std::path::Path;

pub(super) fn target_updated_at(
    agent_type: AgentType,
    session_id: Option<&str>,
    transcript_path: &Path,
    claude_thread: Option<&crate::claude_history::ClaudeThreadRef>,
    gemini_thread: Option<&crate::gemini_history::GeminiThreadRef>,
) -> Option<i64> {
    match agent_type {
        AgentType::Codex => {
            codex_updated_at(session_id).or_else(|| transcript_updated_at(transcript_path))
        }
        AgentType::Claude => claude_thread
            .map(|thread| thread.updated_at)
            .or_else(|| transcript_updated_at(transcript_path)),
        AgentType::Gemini => gemini_thread
            .map(|thread| thread.updated_at)
            .or_else(|| transcript_updated_at(transcript_path)),
        AgentType::OpenCode => {
            opencode_updated_at(session_id).or_else(|| transcript_updated_at(transcript_path))
        }
        _ => transcript_updated_at(transcript_path),
    }
}

fn codex_updated_at(session_id: Option<&str>) -> Option<i64> {
    session_id
        .and_then(|session_id| crate::codex_state::thread_for_id(session_id).ok().flatten())
        .map(|thread| thread.updated_at)
}

fn opencode_updated_at(session_id: Option<&str>) -> Option<i64> {
    session_id
        .and_then(|session_id| {
            crate::opencode_history::thread_for_id(session_id)
                .ok()
                .flatten()
        })
        .map(|thread| thread.updated_at)
}
