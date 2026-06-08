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
use std::path::PathBuf;
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

    let timing = LoadTiming {
        resolve_elapsed,
        parse_elapsed,
        started_at,
    };

    match turns {
        Ok(turns) if !turns.is_empty() => load_non_empty_turns(
            request,
            ParsedSession {
                target,
                transcript_path,
                transcript_updated_at,
                turns,
            },
            timing,
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

struct ParsedSession {
    target: SessionTarget,
    transcript_path: PathBuf,
    transcript_updated_at: Option<i64>,
    turns: Vec<crate::model::PreviewTurn>,
}

#[derive(Clone, Copy)]
struct LoadTiming {
    resolve_elapsed: Duration,
    parse_elapsed: Duration,
    started_at: Instant,
}

fn load_non_empty_turns(
    request: &PreviewRequest,
    parsed: ParsedSession,
    timing: LoadTiming,
) -> Result<SessionPreviewData, String> {
    if let Some(decision) = should_prefer_cached_preview(
        request,
        &parsed.target,
        &parsed.transcript_path,
        parsed.transcript_updated_at,
        parsed.turns.len(),
    ) {
        if decision.prefer_cache {
            log_prefer_cache(request, decision.reason, decision.lag_seconds);
            return Ok(cached_session_preview_with_metadata(
                request,
                Some(parsed.target.origin),
                parsed.target.session_id.clone(),
                Some(parsed.transcript_path.to_string_lossy().to_string()),
                max_i64(parsed.target.updated_at, request.known_updated_at),
            ));
        }
    }

    let transcript_string = parsed.transcript_path.to_string_lossy().to_string();
    persist_resolved_session_if_needed(
        request,
        &parsed.target,
        &parsed.transcript_path,
        &parsed.turns,
    );
    log_parse_success_if_slow(
        request,
        timing.resolve_elapsed,
        timing.parse_elapsed,
        timing.started_at.elapsed(),
        parsed.turns.len(),
    );
    Ok(SessionPreviewData {
        turns: parsed.turns.into(),
        session_origin: parsed.target.origin,
        session_id: parsed.target.session_id,
        transcript_path: Some(transcript_string),
        cache_state: SessionCacheState::Confirmed,
        updated_at: parsed.target.updated_at,
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
