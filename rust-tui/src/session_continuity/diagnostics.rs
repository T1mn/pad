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
