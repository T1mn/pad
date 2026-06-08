use super::model::SessionContinuityRecord;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

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
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64);
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
