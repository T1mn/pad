pub(super) fn pending_request(
    turn_id: Option<&str>,
    phase: &str,
    transcript_path: Option<String>,
    scan_offset: u64,
) -> PendingRequest {
    let accepted_at = (phase != "awaiting_submit").then_some(101);
    PendingRequest {
        request_id: "tg-1".into(),
        chat_id: "1".into(),
        pane_id: "%1".into(),
        agent_kind: "codex".into(),
        target_label: "CODEX • test".into(),
        session_id: Some("s1".into()),
        working_dir: "/tmp/test".into(),
        prompt_text: "hi".into(),
        prompt_hash: "abc".into(),
        turn_id: turn_id.map(str::to_string),
        sent_at: 100,
        sent_at_ms: 100_000,
        accepted_at,
        accepted_at_ms: accepted_at.map(|value| value * 1000),
        last_status_at: None,
        draft_id: 1,
        phase: phase.into(),
        transcript_path,
        result_scan_offset: scan_offset,
        failure_scan_offset: scan_offset,
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

pub(super) fn stop_event(turn_id: Option<&str>, last_assistant_message: &str) -> HookEvent {
    HookEvent {
        event: "stop".into(),
        turn_id: turn_id.map(str::to_string),
        session_id: Some("s1".into()),
        transcript_path: None,
        cwd: None,
        prompt: None,
        last_assistant_message: Some(last_assistant_message.into()),
        timestamp: Some("2026-04-08T00:00:00Z".into()),
        tmux: HookTmuxInfo {
            pane_id: Some("%1".into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: None,
        },
    }
}
