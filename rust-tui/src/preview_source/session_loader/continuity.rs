use crate::preview_source::session_target::SessionTarget;
use crate::preview_source::PreviewRequest;
use std::path::Path;

pub(super) fn should_prefer_cached_preview(
    request: &PreviewRequest,
    target: &SessionTarget,
    transcript_path: &Path,
    transcript_updated_at: Option<i64>,
    transcript_turn_count: usize,
) -> Option<crate::session_continuity::PreviewFallbackDecision> {
    if request.cached_preview_turns.is_empty()
        || request.session_cache_state != Some(crate::model::SessionCacheState::Confirmed)
    {
        return None;
    }

    let decision = crate::session_continuity::assess_preview_fallback(
        crate::session_continuity::PreviewFallbackInput {
            agent_type: &request.agent_type,
            session_id: target
                .session_id
                .as_deref()
                .or(request.agent_session_id.as_deref()),
            transcript_path: Some(transcript_path),
            transcript_updated_at,
            thread_updated_at: target.updated_at,
            known_updated_at: request.known_updated_at,
            cached_turn_count: request.cached_preview_turns.len(),
            transcript_turn_count,
        },
    )?;

    crate::session_continuity::record_preview_assessment(
        &request.agent_type,
        target
            .session_id
            .as_deref()
            .or(request.agent_session_id.as_deref()),
        Some(transcript_path),
        target.updated_at,
        request.cached_preview_turns.len(),
        transcript_turn_count,
        &decision,
    );

    Some(decision)
}
