use super::model::{ContinuityAttemptClassification, ContinuityHealth, SessionContinuityRecord};
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
    let runtime_activity_at = max_ts(record.last_hook_event_at, record.last_hook_cache_persist_at);
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
