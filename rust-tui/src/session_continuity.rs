mod diagnostics;
mod health;
mod model;
mod recording;
mod storage;
mod utils;

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
