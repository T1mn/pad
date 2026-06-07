use super::{
    classify_health, test_classify_preview_health as classify_preview_health,
    test_recompute_record_health as recompute_record_health, ContinuityAttemptClassification,
    ContinuityHealth, PreviewFallbackDecision, SessionContinuityRecord,
};

fn record(session_id: &str) -> SessionContinuityRecord {
    SessionContinuityRecord::new(session_id, 100)
}

#[test]
fn record_becomes_frozen_after_repeated_stale_runtime_activity() {
    let mut record = record("session-1");
    record.last_rollout_mtime = Some(100);
    record.last_hook_event_at = Some(131);
    recompute_record_health(&mut record);
    assert_eq!(record.health, ContinuityHealth::Lagging);
    assert_eq!(record.stale_event_count, 1);

    record.last_hook_cache_persist_at = Some(132);
    recompute_record_health(&mut record);
    assert_eq!(record.health, ContinuityHealth::Frozen);
    assert!(record.lag_seconds.unwrap_or_default() >= 32);
    assert!(record.stale_event_count >= 2);
}

#[test]
fn preview_health_promotes_to_frozen_with_strong_runtime_signal() {
    assert_eq!(
        classify_preview_health(Some(35), 1, Some(140), None),
        ContinuityHealth::Frozen
    );
    assert_eq!(
        classify_preview_health(Some(12), 1, None, None),
        ContinuityHealth::Lagging
    );
    assert_eq!(classify_health(Some(5), 0), ContinuityHealth::Healthy);
}

#[test]
fn bootstrap_classification_clears_once_transcript_is_known() {
    let mut record = record("session-2");
    record.attempt_classification = ContinuityAttemptClassification::TransientResumeBootstrap;
    record.transcript_path = Some("/tmp/demo.jsonl".into());
    super::clear_bootstrap_if_resolved(&mut record);
    assert_eq!(
        record.attempt_classification,
        ContinuityAttemptClassification::Normal
    );
}

#[test]
fn frozen_decision_marks_cache_fallback() {
    let decision = PreviewFallbackDecision {
        prefer_cache: true,
        health: ContinuityHealth::Frozen,
        attempt_classification: ContinuityAttemptClassification::Normal,
        lag_seconds: Some(45),
        reason: "rollout_frozen",
    };
    assert!(decision.prefer_cache);
    assert_eq!(decision.health, ContinuityHealth::Frozen);
    assert_eq!(decision.reason, "rollout_frozen");
}
