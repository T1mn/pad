use super::SessionPreviewData;
use crate::model::{PreviewSessionOrigin, SessionCacheState};
use crate::preview_source::PreviewRequest;

pub(super) fn has_confirmed_cached_preview(request: &PreviewRequest) -> bool {
    !request.cached_preview_turns.is_empty()
        && request.session_cache_state == Some(SessionCacheState::Confirmed)
}

pub(super) fn cache_is_stale(
    known_updated_at: Option<i64>,
    target_updated_at: Option<i64>,
) -> bool {
    match (known_updated_at, target_updated_at) {
        (Some(known), Some(current)) => current > known,
        (None, Some(_)) => true,
        _ => false,
    }
}

pub(super) fn cached_session_preview(request: &PreviewRequest) -> SessionPreviewData {
    cached_session_preview_with_metadata(
        request,
        request.session_origin,
        request.agent_session_id.clone(),
        request.transcript_path.clone(),
        request.known_updated_at,
    )
}

pub(super) fn cached_session_preview_with_metadata(
    request: &PreviewRequest,
    session_origin: Option<PreviewSessionOrigin>,
    session_id: Option<String>,
    transcript_path: Option<String>,
    updated_at: Option<i64>,
) -> SessionPreviewData {
    SessionPreviewData {
        turns: request.cached_preview_turns.clone(),
        session_origin: session_origin.unwrap_or(PreviewSessionOrigin::Pane),
        session_id,
        transcript_path,
        cache_state: request
            .session_cache_state
            .unwrap_or(SessionCacheState::Cached),
        updated_at,
    }
}

pub(super) fn max_i64(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}
