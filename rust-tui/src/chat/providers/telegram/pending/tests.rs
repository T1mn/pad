use super::*;
use crate::session_continuity::{
    ContinuityAttemptClassification, ContinuityHealth, ContinuitySnapshot,
};
use std::fs;

fn sample_pending() -> PendingRequest {
    PendingRequest {
        request_id: "tg-1".into(),
        chat_id: "1".into(),
        pane_id: "%1".into(),
        agent_kind: "codex".into(),
        target_label: "CODEX • 1".into(),
        session_id: Some("session-1".into()),
        working_dir: "/tmp/test".into(),
        prompt_text: "hi".into(),
        prompt_hash: "abc".into(),
        turn_id: Some("turn-1".into()),
        sent_at: 100,
        sent_at_ms: 100_000,
        accepted_at: Some(105),
        accepted_at_ms: Some(105_000),
        last_status_at: None,
        draft_id: 7,
        phase: "awaiting_stop".into(),
        transcript_path: None,
        result_scan_offset: 0,
        failure_scan_offset: 0,
        last_failure_check_at: None,
        approval_scan_offset: 0,
        approval_call_id: None,
        approval_justification: None,
        completed_text: None,
        completed_source: None,
        delivery_attempts: 0,
        delivery_retry_at: 0,
    }
}

#[test]
fn rollout_failure_check_waits_30_seconds_and_then_5_second_backoff() {
    let pending = sample_pending();
    assert!(!super::failures::pending_rollout_failure_check_due(
        &pending, 134
    ));
    assert!(super::failures::pending_rollout_failure_check_due(
        &pending, 135
    ));

    let mut checked = sample_pending();
    checked.last_failure_check_at = Some(135);
    assert!(!super::failures::pending_rollout_failure_check_due(
        &checked, 139
    ));
    assert!(super::failures::pending_rollout_failure_check_due(
        &checked, 140
    ));
}

#[test]
fn detect_pending_rollout_failure_removes_pending_and_updates_scan_offset() {
    let path = std::env::temp_dir().join(format!(
        "pad-telegram-rollout-failure-{}.jsonl",
        std::process::id()
    ));
    let body = concat!(
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"still working\"}]}}\n",
        "{\"type\":\"event_msg\",\"payload\":{\"type\":\"error\",\"message\":\"unexpected status 502 Bad Gateway\",\"codex_error_info\":\"other\"}}\n"
    );
    fs::write(&path, body).unwrap();

    let mut pending = sample_pending();
    pending.transcript_path = Some(path.to_string_lossy().into_owned());
    pending.failure_scan_offset = body.lines().next().unwrap().len() as u64 + 1;
    let mut state = TelegramState {
        pending_requests: vec![pending],
        ..TelegramState::default()
    };

    let resolution =
        super::failures::detect_pending_rollout_failure_for_request(&mut state, "tg-1", 140)
            .unwrap();
    let resolution = resolution.expect("failure resolution");
    assert_eq!(resolution.pending.request_id, "tg-1");
    assert_eq!(
        resolution.failure.message,
        "unexpected status 502 Bad Gateway"
    );
    assert_eq!(resolution.failure.error_info.as_deref(), Some("other"));
    assert!(state.pending_requests.is_empty());

    let _ = fs::remove_file(path);
}

#[test]
fn detect_pending_rollout_failure_updates_last_check_when_no_error_is_found() {
    let path = std::env::temp_dir().join(format!(
        "pad-telegram-rollout-no-failure-{}.jsonl",
        std::process::id()
    ));
    let body = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"still working\"}]}}\n";
    fs::write(&path, body).unwrap();

    let mut pending = sample_pending();
    pending.transcript_path = Some(path.to_string_lossy().into_owned());
    let mut state = TelegramState {
        pending_requests: vec![pending],
        ..TelegramState::default()
    };

    let resolution =
        super::failures::detect_pending_rollout_failure_for_request(&mut state, "tg-1", 140)
            .unwrap();
    assert!(resolution.is_none());
    assert_eq!(state.pending_requests.len(), 1);
    assert_eq!(state.pending_requests[0].last_failure_check_at, Some(140));
    assert_eq!(
        state.pending_requests[0].failure_scan_offset,
        fs::metadata(&path).unwrap().len()
    );

    let _ = fs::remove_file(path);
}

#[test]
fn continuity_status_line_formats_health_and_lag() {
    let snapshot = ContinuitySnapshot {
        session_id: "session-1".into(),
        agent_type: Some("codex".into()),
        transcript_path: Some("/tmp/demo.jsonl".into()),
        last_hook_event: Some("user_prompt_submit".into()),
        last_turn_id: Some("turn-1".into()),
        last_hook_event_at: Some(200),
        last_prompt_submit_at: Some(200),
        last_stop_at: None,
        last_assistant_message_at: None,
        last_hook_cache_persist_at: Some(201),
        last_resolver_sync_at: None,
        last_thread_updated_at: Some(201),
        last_rollout_seen_at: Some(160),
        last_rollout_mtime: Some(160),
        last_rollout_size: Some(12),
        lag_seconds: Some(41),
        stale_event_count: 2,
        bootstrap_event_count: 0,
        health: ContinuityHealth::Frozen,
        attempt_classification: ContinuityAttemptClassification::Normal,
        updated_at: 201,
    };

    let line = super::status::continuity_status_line(crate::i18n::Locale::En, &snapshot);
    assert!(line.contains("Continuity"));
    assert!(line.contains("frozen"));
    assert!(line.contains("41s"));
}

#[test]
fn pending_failure_reply_includes_continuity_details() {
    let pending = sample_pending();
    let failure = crate::chat::approval::CodexFailureEvent {
        message: "unexpected status 502 Bad Gateway".into(),
        error_info: Some("other".into()),
    };
    let snapshot = ContinuitySnapshot {
        session_id: "session-1".into(),
        agent_type: Some("codex".into()),
        transcript_path: Some("/tmp/demo.jsonl".into()),
        last_hook_event: Some("user_prompt_submit".into()),
        last_turn_id: Some("turn-1".into()),
        last_hook_event_at: Some(200),
        last_prompt_submit_at: Some(200),
        last_stop_at: None,
        last_assistant_message_at: None,
        last_hook_cache_persist_at: Some(201),
        last_resolver_sync_at: None,
        last_thread_updated_at: Some(201),
        last_rollout_seen_at: Some(160),
        last_rollout_mtime: Some(160),
        last_rollout_size: Some(12),
        lag_seconds: Some(41),
        stale_event_count: 2,
        bootstrap_event_count: 0,
        health: ContinuityHealth::Frozen,
        attempt_classification: ContinuityAttemptClassification::Normal,
        updated_at: 201,
    };

    let reply = super::failures::pending_failure_reply_text(
        crate::i18n::Locale::En,
        &pending,
        &failure,
        Some(&snapshot),
    );
    assert!(reply.contains("Error kind: other"));
    assert!(reply.contains("Health: frozen"));
    assert!(reply.contains("Lag: 41s"));
    assert!(reply.contains("Transcript: /tmp/demo.jsonl"));
    assert!(reply.ends_with("unexpected status 502 Bad Gateway"));
}
