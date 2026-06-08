use super::cache::{cached_session_preview, cached_session_preview_with_metadata, max_i64};
use super::continuity::should_prefer_cached_preview;
use super::errors::session_unavailable_message;
use super::logging::{
    log_empty_parse_if_slow, log_parse_error_if_slow, log_parse_success_if_slow, log_prefer_cache,
};
use super::parse::parse_session_transcript;
use super::persist::persist_resolved_session_if_needed;
use super::SessionPreviewData;
use crate::i18n::{self, Locale};
use crate::model::SessionCacheState;
use crate::preview_source::session_target::{self, SessionTarget};
use crate::preview_source::PreviewRequest;
use std::time::{Duration, Instant};

pub(super) fn load_resolved_session_preview(
    request: &PreviewRequest,
    locale: Locale,
    target: SessionTarget,
    resolve_elapsed: Duration,
    started_at: Instant,
) -> Result<SessionPreviewData, String> {
    let transcript_path = target.transcript_path.clone();
    let transcript_updated_at = session_target::transcript_updated_at(&transcript_path);
    let parse_started_at = Instant::now();
    let turns = parse_session_transcript(
        &request.agent_type,
        &transcript_path,
        target.session_id.as_deref(),
    );
    let parse_elapsed = parse_started_at.elapsed();

    match turns {
        Ok(turns) if !turns.is_empty() => load_non_empty_turns(
            request,
            target,
            transcript_path,
            transcript_updated_at,
            turns,
            resolve_elapsed,
            parse_elapsed,
            started_at,
        ),
        Ok(_) => handle_empty_parse(request, locale, resolve_elapsed, parse_elapsed, started_at),
        Err(err) => handle_parse_error(
            request,
            locale,
            &err,
            resolve_elapsed,
            parse_elapsed,
            started_at,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn load_non_empty_turns(
    request: &PreviewRequest,
    target: SessionTarget,
    transcript_path: std::path::PathBuf,
    transcript_updated_at: Option<i64>,
    turns: Vec<crate::model::PreviewTurn>,
    resolve_elapsed: Duration,
    parse_elapsed: Duration,
    started_at: Instant,
) -> Result<SessionPreviewData, String> {
    if let Some(decision) = should_prefer_cached_preview(
        request,
        &target,
        &transcript_path,
        transcript_updated_at,
        turns.len(),
    ) {
        if decision.prefer_cache {
            log_prefer_cache(request, decision.reason, decision.lag_seconds);
            return Ok(cached_session_preview_with_metadata(
                request,
                Some(target.origin),
                target.session_id.clone(),
                Some(transcript_path.to_string_lossy().to_string()),
                max_i64(target.updated_at, request.known_updated_at),
            ));
        }
    }

    let transcript_string = transcript_path.to_string_lossy().to_string();
    persist_resolved_session_if_needed(request, &target, &transcript_path, &turns);
    log_parse_success_if_slow(
        request,
        resolve_elapsed,
        parse_elapsed,
        started_at.elapsed(),
        turns.len(),
    );
    Ok(SessionPreviewData {
        turns: turns.into(),
        session_origin: target.origin,
        session_id: target.session_id,
        transcript_path: Some(transcript_string),
        cache_state: SessionCacheState::Confirmed,
        updated_at: target.updated_at,
    })
}

fn handle_empty_parse(
    request: &PreviewRequest,
    locale: Locale,
    resolve_elapsed: Duration,
    parse_elapsed: Duration,
    started_at: Instant,
) -> Result<SessionPreviewData, String> {
    log_empty_parse_if_slow(
        request,
        resolve_elapsed,
        parse_elapsed,
        started_at.elapsed(),
    );
    if !request.cached_preview_turns.is_empty() {
        return Ok(cached_session_preview(request));
    }
    Err(session_unavailable_message(
        locale,
        i18n::t(locale, "preview.session_empty"),
    ))
}

fn handle_parse_error(
    request: &PreviewRequest,
    locale: Locale,
    err: &str,
    resolve_elapsed: Duration,
    parse_elapsed: Duration,
    started_at: Instant,
) -> Result<SessionPreviewData, String> {
    log_parse_error_if_slow(
        request,
        resolve_elapsed,
        parse_elapsed,
        started_at.elapsed(),
    );
    if !request.cached_preview_turns.is_empty() {
        return Ok(cached_session_preview(request));
    }
    Err(session_unavailable_message(
        locale,
        &format!(
            "{}: {}",
            i18n::t(locale, "preview.session_parse_failed"),
            err
        ),
    ))
}
