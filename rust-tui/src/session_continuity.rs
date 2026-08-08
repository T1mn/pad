mod diagnostics {
    use super::model::{
        ContinuityDiagnosticEvent, ContinuityWriteSource, PreviewFallbackDecision,
        SessionContinuityRecord,
    };
    use super::storage::append_diagnostic;
    use crate::hook::HookEvent;

    pub(super) fn append_hook_event_diagnostic(
        now: i64,
        event: &HookEvent,
        record: &SessionContinuityRecord,
    ) {
        append_diagnostic(&ContinuityDiagnosticEvent {
            event: Some(event.event.clone()),
            ..base_event(now, "hook_event", record)
        });
    }

    pub(super) fn append_cache_write_diagnostic(
        now: i64,
        source: ContinuityWriteSource,
        turn_count: usize,
        record: &SessionContinuityRecord,
    ) {
        append_diagnostic(&ContinuityDiagnosticEvent {
            source: Some(source.as_str()),
            cached_turns: Some(turn_count),
            ..base_event(now, "cache_write", record)
        });
    }

    pub(super) fn append_preview_assessment_diagnostic(
        now: i64,
        cached_turn_count: usize,
        transcript_turn_count: usize,
        decision: &PreviewFallbackDecision,
        record: &SessionContinuityRecord,
    ) {
        append_diagnostic(&ContinuityDiagnosticEvent {
            health: decision.health,
            attempt_classification: decision.attempt_classification,
            lag_seconds: decision.lag_seconds,
            cached_turns: Some(cached_turn_count),
            transcript_turns: Some(transcript_turn_count),
            prefer_cache: Some(decision.prefer_cache),
            reason: Some(decision.reason),
            ..base_event(now, "preview_assessment", record)
        });
    }

    fn base_event(
        now: i64,
        kind: &'static str,
        record: &SessionContinuityRecord,
    ) -> ContinuityDiagnosticEvent {
        ContinuityDiagnosticEvent {
            ts: now,
            kind,
            session_id: record.session_id.clone(),
            agent_type: record.agent_type.clone(),
            event: None,
            turn_id: record.last_turn_id.clone(),
            transcript_path: record.transcript_path.clone(),
            source: None,
            health: record.health,
            attempt_classification: record.attempt_classification,
            lag_seconds: record.lag_seconds,
            stale_event_count: record.stale_event_count,
            rollout_mtime: record.last_rollout_mtime,
            rollout_size: record.last_rollout_size,
            thread_updated_at: record.last_thread_updated_at,
            cached_turns: None,
            transcript_turns: None,
            prefer_cache: None,
            reason: None,
        }
    }
}
mod health {
    use super::model::{
        ContinuityAttemptClassification, ContinuityHealth, SessionContinuityRecord,
    };
    use super::utils::{lag_seconds, max_ts};

    const LAGGING_THRESHOLD_SECS: i64 = 10;
    const FROZEN_THRESHOLD_SECS: i64 = 30;

    pub(super) fn clear_bootstrap_if_resolved(record: &mut SessionContinuityRecord) {
        if record.transcript_path.is_some()
            || record.last_hook_cache_persist_at.is_some()
            || record.last_resolver_sync_at.is_some()
        {
            record.attempt_classification = ContinuityAttemptClassification::Normal;
        }
    }

    pub(super) fn recompute_record_health(record: &mut SessionContinuityRecord) {
        let runtime_activity_at =
            max_ts(record.last_hook_event_at, record.last_hook_cache_persist_at);
        let lag_seconds = lag_seconds(runtime_activity_at, record.last_rollout_mtime);
        record.stale_event_count = next_stale_event_count(record.stale_event_count, lag_seconds);
        record.lag_seconds = lag_seconds;
        record.health = classify_health(lag_seconds, record.stale_event_count);
    }

    fn next_stale_event_count(current: u32, lag_seconds: Option<i64>) -> u32 {
        match lag_seconds {
            Some(lag_seconds) if lag_seconds >= LAGGING_THRESHOLD_SECS => {
                current.saturating_add(1).max(1)
            }
            _ => 0,
        }
    }

    pub(super) fn classify_health(
        lag_seconds: Option<i64>,
        stale_event_count: u32,
    ) -> ContinuityHealth {
        match lag_seconds {
            Some(lag_seconds) if lag_seconds >= FROZEN_THRESHOLD_SECS && stale_event_count >= 2 => {
                ContinuityHealth::Frozen
            }
            Some(lag_seconds) if lag_seconds >= LAGGING_THRESHOLD_SECS => ContinuityHealth::Lagging,
            _ => ContinuityHealth::Healthy,
        }
    }

    pub(super) fn classify_preview_health(
        lag_seconds: Option<i64>,
        stale_event_count: u32,
        thread_updated_at: Option<i64>,
        known_updated_at: Option<i64>,
    ) -> ContinuityHealth {
        match lag_seconds {
            Some(lag_seconds)
                if lag_seconds >= FROZEN_THRESHOLD_SECS
                    && (stale_event_count >= 2
                        || thread_updated_at.is_some()
                        || known_updated_at.is_some()) =>
            {
                ContinuityHealth::Frozen
            }
            Some(lag_seconds) if lag_seconds >= LAGGING_THRESHOLD_SECS => ContinuityHealth::Lagging,
            _ => ContinuityHealth::Healthy,
        }
    }
}
mod model;
mod recording;
mod storage;
mod utils {
    use super::model::SessionContinuityRecord;
    use std::fs;
    use std::path::Path;

    pub(super) fn observe_transcript(
        record: &mut SessionContinuityRecord,
        transcript_path: Option<&Path>,
        now: i64,
    ) {
        let Some(transcript_path) = transcript_path else {
            return;
        };
        record.transcript_path = Some(transcript_path.to_string_lossy().to_string());
        record.last_rollout_seen_at = Some(now);

        let Ok(metadata) = fs::metadata(transcript_path) else {
            return;
        };
        record.last_rollout_size = Some(metadata.len());
        record.last_rollout_mtime = metadata
            .modified()
            .ok()
            .and_then(crate::time::system_time_unix_secs);
    }

    pub(super) fn clean_text(value: Option<&str>) -> Option<&str> {
        value.map(str::trim).filter(|text| !text.is_empty())
    }

    pub(super) fn max_ts(left: Option<i64>, right: Option<i64>) -> Option<i64> {
        match (left, right) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        }
    }

    pub(super) fn lag_seconds(
        runtime_activity_at: Option<i64>,
        rollout_mtime: Option<i64>,
    ) -> Option<i64> {
        let runtime_activity_at = runtime_activity_at?;
        let rollout_mtime = rollout_mtime?;
        (runtime_activity_at > rollout_mtime).then_some(runtime_activity_at - rollout_mtime)
    }

    pub(super) fn now_ts() -> i64 {
        crate::time::unix_now_ts()
    }
}

use crate::model::AgentType;
use std::sync::{Mutex, OnceLock};

pub use model::{
    ContinuityAttemptClassification, ContinuityHealth, ContinuitySnapshot, ContinuityWriteSource,
    PreviewFallbackDecision, PreviewFallbackInput,
};
pub use recording::{record_cache_write, record_hook_event, record_preview_assessment};

use health::classify_preview_health;
use storage::load_record_snapshot;
use utils::{clean_text, lag_seconds, max_ts};

#[cfg(test)]
use health::{
    classify_health, classify_preview_health as test_classify_preview_health,
    clear_bootstrap_if_resolved, recompute_record_health as test_recompute_record_health,
};
#[cfg(test)]
use model::SessionContinuityRecord;

const CONTINUITY_VERSION: u32 = 1;
static CONTINUITY_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn assess_preview_fallback(input: PreviewFallbackInput<'_>) -> Option<PreviewFallbackDecision> {
    if input.cached_turn_count == 0 {
        return None;
    }
    let session_id = clean_text(input.session_id)?;
    let snapshot = load_record_snapshot(session_id)?;
    let runtime_activity_at = max_ts(
        snapshot.last_hook_event_at,
        max_ts(
            snapshot.last_hook_cache_persist_at,
            max_ts(input.thread_updated_at, input.known_updated_at),
        ),
    );
    let rollout_mtime = input.transcript_updated_at.or(snapshot.last_rollout_mtime);
    let lag_seconds = lag_seconds(runtime_activity_at, rollout_mtime);
    let health = classify_preview_health(
        lag_seconds,
        snapshot.stale_event_count,
        input.thread_updated_at,
        input.known_updated_at,
    );
    let prefer_cache = health == ContinuityHealth::Frozen && input.transcript_turn_count > 0;
    let reason = if prefer_cache {
        "rollout_frozen"
    } else if health == ContinuityHealth::Lagging {
        "rollout_lagging"
    } else if snapshot.attempt_classification
        == ContinuityAttemptClassification::TransientResumeBootstrap
    {
        "transient_resume_bootstrap"
    } else {
        "healthy"
    };

    if health == ContinuityHealth::Healthy
        && snapshot.attempt_classification == ContinuityAttemptClassification::Normal
    {
        return None;
    }

    if !matches!(
        input.agent_type,
        AgentType::Codex | AgentType::Claude | AgentType::Gemini
    ) {
        return None;
    }

    let _ = input.transcript_path;

    Some(PreviewFallbackDecision {
        prefer_cache,
        health,
        attempt_classification: snapshot.attempt_classification,
        lag_seconds,
        reason,
    })
}

pub fn load_snapshot_for(
    session_id: Option<&str>,
    transcript_path: Option<&str>,
) -> Option<ContinuitySnapshot> {
    storage::load_snapshot_for(session_id, transcript_path)
}

#[cfg(test)]
mod tests;
