use super::*;
use crate::hook::HookTmuxInfo;
use std::fs;

#[test]
fn codex_stop_prefers_transcript_completion_over_stale_hook_payload() {
    let path = std::env::temp_dir().join(format!(
        "pad-codex-stop-prefer-transcript-{}.jsonl",
        std::process::id()
    ));
    let old = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"old answer\"}]}}\n";
    let new = concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"final_answer\",\"content\":[{\"type\":\"output_text\",\"text\":\"new answer\"}]}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\",\"turn_id\":\"turn-new\",\"last_agent_message\":\"new answer\"}}\n"
        );
    fs::write(&path, format!("{old}{new}")).unwrap();

    let pending = PendingRequest {
        request_id: "tg-1".into(),
        chat_id: "1".into(),
        pane_id: "%1".into(),
        agent_kind: "codex".into(),
        target_label: "CODEX • test".into(),
        session_id: Some("s1".into()),
        working_dir: "/tmp/test".into(),
        prompt_text: "hi".into(),
        prompt_hash: "abc".into(),
        turn_id: Some("turn-new".into()),
        sent_at: 100,
        sent_at_ms: 100_000,
        accepted_at: Some(101),
        accepted_at_ms: Some(101_000),
        last_status_at: None,
        draft_id: 1,
        phase: "awaiting_stop".into(),
        transcript_path: Some(path.to_string_lossy().into_owned()),
        result_scan_offset: old.len() as u64,
        failure_scan_offset: old.len() as u64,
        last_failure_check_at: None,
        approval_scan_offset: 0,
        approval_call_id: None,
        approval_justification: None,
        completed_text: None,
        completed_source: None,
        delivery_attempts: 0,
        delivery_retry_at: 0,
    };
    let event = HookEvent {
        event: "stop".into(),
        turn_id: Some("turn-old".into()),
        session_id: Some("s1".into()),
        transcript_path: pending.transcript_path.clone(),
        cwd: None,
        prompt: None,
        last_assistant_message: Some("stale hook payload".into()),
        timestamp: Some("2026-04-07T00:00:00Z".into()),
        tmux: HookTmuxInfo {
            pane_id: Some("%1".into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: None,
        },
    };

    let resolved = resolve_pending_result_text(&pending, &event);
    assert_eq!(resolved.source, "transcript_completion");
    assert_eq!(resolved.text.as_deref(), Some("new answer"));

    let _ = fs::remove_file(path);
}

#[test]
fn pending_turn_must_match_stop_turn_when_both_exist() {
    let pending = PendingRequest {
        request_id: "tg-1".into(),
        chat_id: "1".into(),
        pane_id: "%1".into(),
        agent_kind: "codex".into(),
        target_label: "CODEX • test".into(),
        session_id: Some("s1".into()),
        working_dir: "/tmp/test".into(),
        prompt_text: "hi".into(),
        prompt_hash: "abc".into(),
        turn_id: Some("turn-a".into()),
        sent_at: 100,
        sent_at_ms: 100_000,
        accepted_at: Some(101),
        accepted_at_ms: Some(101_000),
        last_status_at: None,
        draft_id: 1,
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
    };
    let event = HookEvent {
        event: "stop".into(),
        turn_id: Some("turn-b".into()),
        session_id: Some("s1".into()),
        transcript_path: None,
        cwd: None,
        prompt: None,
        last_assistant_message: Some("wrong turn".into()),
        timestamp: Some("2026-04-07T00:00:00Z".into()),
        tmux: HookTmuxInfo {
            pane_id: Some("%1".into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: None,
        },
    };

    assert!(!hook_event_matches_pending_turn(&pending, &event));
}

#[test]
fn codex_stop_without_turn_id_is_ignored_when_pending_turn_exists() {
    let pending = PendingRequest {
        request_id: "tg-1".into(),
        chat_id: "1".into(),
        pane_id: "%1".into(),
        agent_kind: "codex".into(),
        target_label: "CODEX • test".into(),
        session_id: Some("s1".into()),
        working_dir: "/tmp/test".into(),
        prompt_text: "hi".into(),
        prompt_hash: "abc".into(),
        turn_id: Some("turn-a".into()),
        sent_at: 100,
        sent_at_ms: 100_000,
        accepted_at: Some(101),
        accepted_at_ms: Some(101_000),
        last_status_at: None,
        draft_id: 1,
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
    };
    let event = HookEvent {
        event: "stop".into(),
        turn_id: None,
        session_id: Some("s1".into()),
        transcript_path: None,
        cwd: None,
        prompt: None,
        last_assistant_message: Some("missing turn".into()),
        timestamp: Some("2026-04-08T00:00:00Z".into()),
        tmux: HookTmuxInfo {
            pane_id: Some("%1".into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: None,
        },
    };

    assert!(!hook_event_matches_pending_turn(&pending, &event));
}

#[test]
fn stop_is_ignored_while_pending_still_awaits_submit() {
    let pending = PendingRequest {
        request_id: "tg-1".into(),
        chat_id: "1".into(),
        pane_id: "%1".into(),
        agent_kind: "codex".into(),
        target_label: "CODEX • test".into(),
        session_id: Some("s1".into()),
        working_dir: "/tmp/test".into(),
        prompt_text: "hi".into(),
        prompt_hash: "abc".into(),
        turn_id: None,
        sent_at: 100,
        sent_at_ms: 100_000,
        accepted_at: None,
        accepted_at_ms: None,
        last_status_at: None,
        draft_id: 1,
        phase: "awaiting_submit".into(),
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
    };
    let event = HookEvent {
        event: "stop".into(),
        turn_id: Some("turn-old".into()),
        session_id: Some("s1".into()),
        transcript_path: None,
        cwd: None,
        prompt: None,
        last_assistant_message: Some("old answer".into()),
        timestamp: Some("2026-04-08T00:00:00Z".into()),
        tmux: HookTmuxInfo {
            pane_id: Some("%1".into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: None,
        },
    };

    assert!(!pending_can_complete_from_stop(&pending, &event));
}
