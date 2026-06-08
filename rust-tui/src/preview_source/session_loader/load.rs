use super::cache::{cache_is_stale, cached_session_preview, has_confirmed_cached_preview};
use super::errors::session_unavailable_message;
use super::logging::log_missing_target_if_slow;
use super::resolved::load_resolved_session_preview;
use super::SessionPreviewData;
use crate::i18n::{self, Locale};
use crate::preview_source::{session_target, PreviewRequest};
use std::time::Instant;

pub(in crate::preview_source) fn load_session_preview(
    request: &PreviewRequest,
    locale: Locale,
) -> Result<SessionPreviewData, String> {
    let started_at = Instant::now();
    if has_confirmed_cached_preview(request) && request.known_updated_at.is_some() {
        return Ok(cached_session_preview(request));
    }

    let resolve_started_at = Instant::now();
    let target = session_target::resolve_session_target(request);
    let resolve_elapsed = resolve_started_at.elapsed();
    let target_updated_at = target.as_ref().and_then(|target| target.updated_at);
    if has_confirmed_cached_preview(request)
        && !cache_is_stale(request.known_updated_at, target_updated_at)
    {
        return Ok(cached_session_preview(request));
    }

    if let Some(target) = target {
        return load_resolved_session_preview(request, locale, target, resolve_elapsed, started_at);
    }

    if !request.cached_preview_turns.is_empty() {
        return Ok(cached_session_preview(request));
    }

    log_missing_target_if_slow(request, resolve_elapsed, started_at.elapsed());
    Err(session_unavailable_message(
        locale,
        i18n::t(locale, "preview.session_missing"),
    ))
}
