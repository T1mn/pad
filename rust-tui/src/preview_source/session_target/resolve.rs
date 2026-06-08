mod panel;
mod path;
mod updated_at;

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
