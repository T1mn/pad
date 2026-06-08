use super::support::{pending_request, stop_event};
use std::fs;

#[test]
fn codex_stop_prefers_transcript_completion_over_stale_hook_payload() {
    let path = crate::test_support::temp_path("pad-codex-stop", "prefer-transcript").with_extension("jsonl");
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
