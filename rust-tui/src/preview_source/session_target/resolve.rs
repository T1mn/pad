mod panel {
    use super::super::target::SessionTarget;
    use crate::model::{AgentPanel, AgentState};
    use crate::preview_source::PreviewRequest;

    pub(crate) fn persistence_panel_from_request(
        request: &PreviewRequest,
        target: &SessionTarget,
    ) -> Option<AgentPanel> {
        let pane_id = request.live_pane_id.clone()?;
        Some(AgentPanel {
            session: String::new(),
            window: String::new(),
            window_index: String::new(),
            pane: String::new(),
            pane_id,
            agent_type: request.agent_type.clone(),
            working_dir: request.working_dir.clone(),
            is_active: matches!(request.state, AgentState::Busy | AgentState::Waiting),
            state: request.state.clone(),
            transcript_path: Some(target.transcript_path.to_string_lossy().to_string()),
            cached_preview_turns: request.cached_preview_turns.clone(),
            session_cache_state: request.session_cache_state,
            agent_session_id: target.session_id.clone(),
            last_user_prompt: request
                .cached_preview_turns
                .first()
                .map(|turn| turn.question.clone()),
            last_assistant_message: request
                .cached_preview_turns
                .first()
                .and_then(|turn| turn.answer.clone()),
            has_unread_stop: false,
        })
    }
}
mod path {
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
            AgentType::Claude => {
                claude_transcript_path(original_request, session_id, claude_thread)
            }
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
                (name.ends_with(".jsonl") || name.ends_with(".jsonl.zst"))
                    && name.contains(session_id)
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
}
mod updated_at {
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
}

use super::sources::{
    claude_thread_for_session_id, gemini_thread_for_request, resolved_session_id_for_request,
};
use super::target::SessionTarget;
use crate::model::AgentType;
use crate::preview_source::PreviewRequest;
use std::time::Instant;

pub(crate) use panel::persistence_panel_from_request;

pub(crate) fn resolve_session_target(request: &PreviewRequest) -> Option<SessionTarget> {
    let started_at = Instant::now();
    let gemini_thread = if request.agent_type == AgentType::Gemini {
        gemini_thread_for_request(request)
    } else {
        None
    };
    let resolved_session_id = resolved_session_id_for_request(request, gemini_thread.as_ref());
    let claude_thread = if request.agent_type == AgentType::Claude {
        resolved_session_id
            .as_deref()
            .and_then(claude_thread_for_session_id)
    } else {
        None
    };
    let transcript_path = path::resolve_transcript_path(
        request,
        &PreviewRequest {
            agent_session_id: resolved_session_id.clone(),
            ..request.clone()
        },
        claude_thread.as_ref(),
        gemini_thread.as_ref(),
    )?;
    let updated_at = updated_at::target_updated_at(
        request.agent_type.clone(),
        resolved_session_id.as_deref(),
        &transcript_path,
        claude_thread.as_ref(),
        gemini_thread.as_ref(),
    );

    Some(SessionTarget {
        origin: request
            .session_origin
            .unwrap_or(crate::model::PreviewSessionOrigin::Pane),
        session_id: resolved_session_id,
        transcript_path,
        updated_at,
    })
    .inspect(|target| {
        if started_at.elapsed().as_millis() >= 15 {
            crate::log_debug!(
                "session_target.resolve: target={} agent={} elapsed_ms={} path={}",
                request.target_key,
                request.agent_type,
                started_at.elapsed().as_millis(),
                target.transcript_path.display()
            );
        }
    })
}
