use super::*;
use crate::hook::HookTerminalInfo;

pub(crate) mod completion {
    use super::support::{pending_request, stop_event};
    use super::*;
    use std::fs;

    pub(crate) fn codex_stop_prefers_transcript_completion_over_stale_hook_payload() {
        let path = crate::test_support::temp_path("pad-codex-stop", "prefer-transcript")
            .with_extension("jsonl");
        let old = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"old answer\"}]}}\n";
        let new = concat!(
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"final_answer\",\"content\":[{\"type\":\"output_text\",\"text\":\"new answer\"}]}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\",\"turn_id\":\"turn-new\",\"last_agent_message\":\"new answer\"}}\n"
        );
        fs::write(&path, format!("{old}{new}")).unwrap();

        let pending = pending_request(
            Some("turn-new"),
            "awaiting_stop",
            Some(path.to_string_lossy().into_owned()),
            old.len() as u64,
        );
        let mut event = stop_event(Some("turn-old"), "stale hook payload");
        event.transcript_path = pending.transcript_path.clone();
        event.timestamp = Some("2026-04-07T00:00:00Z".into());

        let resolved = resolve_pending_result_text(&pending, &event);
        assert_eq!(resolved.source, "transcript_completion");
        assert_eq!(resolved.text.as_deref(), Some("new answer"));

        let _ = fs::remove_file(path);
    }
}

pub(crate) mod phase_gate {
    use super::support::{pending_request, stop_event};
    use super::*;

    pub(crate) fn stop_is_ignored_while_pending_still_awaits_submit() {
        let pending = pending_request(None, "awaiting_submit", None, 0);
        let event = stop_event(Some("turn-old"), "old answer");

        assert!(!pending_can_complete_from_stop(&pending, &event));
    }
}

mod support {
    use super::*;
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
            terminal: HookTerminalInfo {
                pane_id: Some("%1".into()),
                session_name: Some("0".into()),
                window_index: Some("1".into()),
                pane_index: Some("1".into()),
                pane_current_path: None,
            },
        }
    }
}

pub(crate) mod turn_match {
    use super::support::{pending_request, stop_event};
    use super::*;

    pub(crate) fn pending_turn_must_match_stop_turn_when_both_exist() {
        let pending = pending_request(Some("turn-a"), "awaiting_stop", None, 0);
        let mut event = stop_event(Some("turn-b"), "wrong turn");
        event.timestamp = Some("2026-04-07T00:00:00Z".into());

        assert!(!hook_event_matches_pending_turn(&pending, &event));
    }

    pub(crate) fn codex_stop_without_turn_id_is_ignored_when_pending_turn_exists() {
        let pending = pending_request(Some("turn-a"), "awaiting_stop", None, 0);
        let event = stop_event(None, "missing turn");

        assert!(!hook_event_matches_pending_turn(&pending, &event));
    }
}
