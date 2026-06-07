use super::{claude, codex, gemini, opencode, session_target, PreviewRequest, SessionReadMode};
use crate::i18n::{self, Locale};
use crate::model::{AgentType, PreviewSessionOrigin, SessionCacheState, SharedPreviewTurns};
use std::time::Instant;

pub(super) struct SessionPreviewData {
    pub(super) turns: SharedPreviewTurns,
    pub(super) session_origin: PreviewSessionOrigin,
    pub(super) session_id: Option<String>,
    pub(super) transcript_path: Option<String>,
    pub(super) cache_state: SessionCacheState,
    pub(super) updated_at: Option<i64>,
}

pub(super) fn load_session_preview(
    request: &PreviewRequest,
    locale: Locale,
) -> Result<SessionPreviewData, String> {
    let started_at = Instant::now();
    let can_use_cached = !request.cached_preview_turns.is_empty()
        && request.session_cache_state == Some(SessionCacheState::Confirmed)
        && request.known_updated_at.is_some();

    if can_use_cached {
        return Ok(cached_session_preview(request));
    }

    let resolve_started_at = Instant::now();
    let target = session_target::resolve_session_target(request);
    let resolve_elapsed = resolve_started_at.elapsed();
    let target_updated_at = target.as_ref().and_then(|target| target.updated_at);
    let cache_stale = match (request.known_updated_at, target_updated_at) {
        (Some(known), Some(current)) => current > known,
        (None, Some(_)) => true,
        _ => false,
    };
    if !request.cached_preview_turns.is_empty()
        && request.session_cache_state == Some(SessionCacheState::Confirmed)
        && !cache_stale
    {
        return Ok(cached_session_preview(request));
    }

    if let Some(target) = target {
        let transcript_path = target.transcript_path.clone();
        let transcript_updated_at = session_target::transcript_updated_at(&transcript_path);
        let parse_started_at = Instant::now();
        let turns = match request.agent_type {
            AgentType::Codex => {
                codex::parse_transcript(&transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::Claude => {
                claude::parse_transcript(&transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::Gemini => {
                gemini::parse_transcript(&transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::OpenCode => opencode::parse_transcript(
                &transcript_path,
                target.session_id.as_deref(),
                SessionReadMode::FullBackfill,
            ),
            _ => Ok(Vec::new()),
        };
        let parse_elapsed = parse_started_at.elapsed();

        match turns {
            Ok(turns) if !turns.is_empty() => {
                let continuity_decision = if !request.cached_preview_turns.is_empty()
                    && request.session_cache_state == Some(SessionCacheState::Confirmed)
                {
                    crate::session_continuity::assess_preview_fallback(
                        crate::session_continuity::PreviewFallbackInput {
                            agent_type: &request.agent_type,
                            session_id: target
                                .session_id
                                .as_deref()
                                .or(request.agent_session_id.as_deref()),
                            transcript_path: Some(&transcript_path),
                            transcript_updated_at,
                            thread_updated_at: target.updated_at,
                            known_updated_at: request.known_updated_at,
                            cached_turn_count: request.cached_preview_turns.len(),
                            transcript_turn_count: turns.len(),
                        },
                    )
                } else {
                    None
                };

                if let Some(decision) = continuity_decision.as_ref() {
                    crate::session_continuity::record_preview_assessment(
                        &request.agent_type,
                        target
                            .session_id
                            .as_deref()
                            .or(request.agent_session_id.as_deref()),
                        Some(&transcript_path),
                        target.updated_at,
                        request.cached_preview_turns.len(),
                        turns.len(),
                        decision,
                    );

                    if decision.prefer_cache {
                        log_debug!(
                            "preview.session: target={} agent={} continuity={} lag_s={} prefer_cache=1",
                            request.target_key,
                            request.agent_type,
                            decision.reason,
                            decision.lag_seconds.unwrap_or_default()
                        );
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
                if target.origin == PreviewSessionOrigin::Pane && request.persist_resolved_session {
                    if let Some(panel) =
                        session_target::persistence_panel_from_request(request, &target)
                    {
                        if let Err(err) = crate::session_cache::persist_resolved_session(
                            &panel,
                            &transcript_path,
                            &turns,
                        ) {
                            log_debug!("session_cache: persist resolved failed: {}", err);
                        }
                    }
                }
                let total_elapsed = started_at.elapsed();
                if total_elapsed.as_millis() >= 40 {
                    log_debug!(
                        "preview.session: target={} agent={} resolve_ms={} parse_ms={} turns={} total_ms={}",
                        request.target_key,
                        request.agent_type,
                        resolve_elapsed.as_millis(),
                        parse_elapsed.as_millis(),
                        turns.len(),
                        total_elapsed.as_millis()
                    );
                }
                return Ok(SessionPreviewData {
                    turns: turns.into(),
                    session_origin: target.origin,
                    session_id: target.session_id,
                    transcript_path: Some(transcript_string),
                    cache_state: SessionCacheState::Confirmed,
                    updated_at: target.updated_at,
                });
            }
            Ok(_) => {
                if started_at.elapsed().as_millis() >= 40 {
                    log_debug!(
                        "preview.session: target={} agent={} resolve_ms={} parse_ms={} turns=0 total_ms={}",
                        request.target_key,
                        request.agent_type,
                        resolve_elapsed.as_millis(),
                        parse_elapsed.as_millis(),
                        started_at.elapsed().as_millis()
                    );
                }
                if !request.cached_preview_turns.is_empty() {
                    return Ok(cached_session_preview(request));
                }
                return Err(session_unavailable_message(
                    locale,
                    i18n::t(locale, "preview.session_empty"),
                ));
            }
            Err(err) => {
                if started_at.elapsed().as_millis() >= 40 {
                    log_debug!(
                        "preview.session: target={} agent={} resolve_ms={} parse_ms={} error=1 total_ms={}",
                        request.target_key,
                        request.agent_type,
                        resolve_elapsed.as_millis(),
                        parse_elapsed.as_millis(),
                        started_at.elapsed().as_millis()
                    );
                }
                if !request.cached_preview_turns.is_empty() {
                    return Ok(cached_session_preview(request));
                }
                return Err(session_unavailable_message(
                    locale,
                    &format!(
                        "{}: {}",
                        i18n::t(locale, "preview.session_parse_failed"),
                        err
                    ),
                ));
            }
        }
    }

    if !request.cached_preview_turns.is_empty() {
        return Ok(cached_session_preview(request));
    }

    if started_at.elapsed().as_millis() >= 20 {
        log_debug!(
            "preview.session: target={} agent={} missing_target=1 resolve_ms={} total_ms={}",
            request.target_key,
            request.agent_type,
            resolve_elapsed.as_millis(),
            started_at.elapsed().as_millis()
        );
    }

    Err(session_unavailable_message(
        locale,
        i18n::t(locale, "preview.session_missing"),
    ))
}

fn cached_session_preview(request: &PreviewRequest) -> SessionPreviewData {
    cached_session_preview_with_metadata(
        request,
        request.session_origin,
        request.agent_session_id.clone(),
        request.transcript_path.clone(),
        request.known_updated_at,
    )
}

fn cached_session_preview_with_metadata(
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

fn max_i64(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn session_unavailable_message(locale: Locale, detail: &str) -> String {
    format!(
        "{}\n\n{}",
        i18n::t(locale, "preview.session_unavailable"),
        detail
    )
}
