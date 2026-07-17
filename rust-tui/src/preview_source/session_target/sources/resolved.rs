use super::codex::codex_thread_for_working_dir;
use super::opencode::opencode_thread_for_working_dir;
use crate::model::{AgentState, AgentType};
use crate::preview_source::PreviewRequest;
use std::path::Path;

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
                    super::super::super::gemini::extract_session_id_from_transcript(Path::new(path))
                })
            });
    }

    request
        .agent_session_id
        .clone()
        .or_else(|| {
            if request.agent_type == AgentType::Grok {
                request
                    .transcript_path
                    .as_deref()
                    .and_then(|path| Path::new(path).parent())
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .map(str::to_string)
            } else {
                None
            }
        })
        .or_else(|| {
            if request.transcript_path.is_none()
                && request.agent_type == AgentType::Codex
                && request.state == AgentState::Idle
            {
                request
                    .live_pane_id
                    .as_deref()
                    .and_then(crate::preview_source::codex::resolve_live_session_id)
            } else {
                None
            }
        })
        .or_else(|| {
            let require_unique = request.live_pane_id.is_some();
            if request.agent_type == AgentType::Codex {
                codex_thread_for_working_dir(&request.working_dir, require_unique)
                    .map(|thread| thread.thread_id)
            } else if request.agent_type == AgentType::OpenCode {
                opencode_thread_for_working_dir(&request.working_dir, require_unique)
                    .map(|thread| thread.session_id)
            } else {
                None
            }
        })
}
