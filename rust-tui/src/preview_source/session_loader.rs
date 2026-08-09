mod cache {
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
}
mod continuity {
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
}
mod errors {
    use crate::i18n::{self, Locale};

    pub(super) fn session_unavailable_message(locale: Locale, detail: &str) -> String {
        format!(
            "{}\n\n{}",
            i18n::t(locale, "preview.session_unavailable"),
            detail
        )
    }
}
mod load {
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
            return load_resolved_session_preview(
                request,
                locale,
                target,
                resolve_elapsed,
                started_at,
            );
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
}
mod logging {
    use crate::preview_source::PreviewRequest;
    use std::time::Duration;

    pub(super) fn log_prefer_cache(
        request: &PreviewRequest,
        reason: &str,
        lag_seconds: Option<i64>,
    ) {
        log_debug!(
            "preview.session: target={} agent={} continuity={} lag_s={} prefer_cache=1",
            request.target_key,
            request.agent_type,
            reason,
            lag_seconds.unwrap_or_default()
        );
    }

    pub(super) fn log_parse_success_if_slow(
        request: &PreviewRequest,
        resolve_elapsed: Duration,
        parse_elapsed: Duration,
        total_elapsed: Duration,
        turns_len: usize,
    ) {
        if total_elapsed.as_millis() >= 40 {
            log_debug!(
                "preview.session: target={} agent={} resolve_ms={} parse_ms={} turns={} total_ms={}",
                request.target_key,
                request.agent_type,
                resolve_elapsed.as_millis(),
                parse_elapsed.as_millis(),
                turns_len,
                total_elapsed.as_millis()
            );
        }
    }

    pub(super) fn log_empty_parse_if_slow(
        request: &PreviewRequest,
        resolve_elapsed: Duration,
        parse_elapsed: Duration,
        total_elapsed: Duration,
    ) {
        if total_elapsed.as_millis() >= 40 {
            log_debug!(
                "preview.session: target={} agent={} resolve_ms={} parse_ms={} turns=0 total_ms={}",
                request.target_key,
                request.agent_type,
                resolve_elapsed.as_millis(),
                parse_elapsed.as_millis(),
                total_elapsed.as_millis()
            );
        }
    }

    pub(super) fn log_parse_error_if_slow(
        request: &PreviewRequest,
        resolve_elapsed: Duration,
        parse_elapsed: Duration,
        total_elapsed: Duration,
    ) {
        if total_elapsed.as_millis() >= 40 {
            log_debug!(
                "preview.session: target={} agent={} resolve_ms={} parse_ms={} error=1 total_ms={}",
                request.target_key,
                request.agent_type,
                resolve_elapsed.as_millis(),
                parse_elapsed.as_millis(),
                total_elapsed.as_millis()
            );
        }
    }

    pub(super) fn log_missing_target_if_slow(
        request: &PreviewRequest,
        resolve_elapsed: Duration,
        total_elapsed: Duration,
    ) {
        if total_elapsed.as_millis() >= 20 {
            log_debug!(
                "preview.session: target={} agent={} missing_target=1 resolve_ms={} total_ms={}",
                request.target_key,
                request.agent_type,
                resolve_elapsed.as_millis(),
                total_elapsed.as_millis()
            );
        }
    }
}
mod parse {
    use super::super::{claude, codex, gemini, grok, opencode, SessionReadMode};
    use crate::model::{AgentType, PreviewTurn};
    use std::path::Path;

    pub(super) fn parse_session_transcript(
        agent_type: &AgentType,
        transcript_path: &Path,
        session_id: Option<&str>,
    ) -> Result<Vec<PreviewTurn>, String> {
        match agent_type {
            AgentType::Codex => {
                codex::parse_transcript(transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::Claude => {
                claude::parse_transcript(transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::Gemini => {
                gemini::parse_transcript(transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::Grok => {
                grok::parse_transcript(transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::OpenCode => opencode::parse_transcript(
                transcript_path,
                session_id,
                SessionReadMode::FullBackfill,
            ),
            _ => Ok(Vec::new()),
        }
    }
}
mod persist {
    use crate::model::{PreviewSessionOrigin, PreviewTurn};
    use crate::preview_source::session_target::{self, SessionTarget};
    use crate::preview_source::PreviewRequest;
    use std::path::Path;

    pub(super) fn persist_resolved_session_if_needed(
        request: &PreviewRequest,
        target: &SessionTarget,
        transcript_path: &Path,
        turns: &[PreviewTurn],
    ) {
        if target.origin != PreviewSessionOrigin::Pane || !request.persist_resolved_session {
            return;
        }

        let Some(panel) = session_target::persistence_panel_from_request(request, target) else {
            return;
        };

        if let Err(err) =
            crate::session_cache::persist_resolved_session(&panel, transcript_path, turns)
        {
            log_debug!("session_cache: persist resolved failed: {}", err);
        }
    }
}
mod resolved;

use crate::model::{PreviewSessionOrigin, SessionCacheState, SharedPreviewTurns};

pub(super) use load::load_session_preview;

pub(super) struct SessionPreviewData {
    pub(super) turns: SharedPreviewTurns,
    pub(super) session_origin: PreviewSessionOrigin,
    pub(super) session_id: Option<String>,
    pub(super) transcript_path: Option<String>,
    pub(super) cache_state: SessionCacheState,
    pub(super) updated_at: Option<i64>,
}
