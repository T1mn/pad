use super::*;

#[test]
fn codex_approval_scan_tracks_open_and_resolved_requests() {
    let path =
        std::env::temp_dir().join(format!("pad-codex-approval-{}.jsonl", std::process::id()));
    let body = concat!(
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call_old\",\"arguments\":\"{\\\"sandbox_permissions\\\":\\\"require_escalated\\\",\\\"justification\\\":\\\"old\\\"}\"}}\n",
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"call_old\",\"output\":\"ok\"}}\n",
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"call_new\",\"arguments\":\"{\\\"sandbox_permissions\\\":\\\"require_escalated\\\",\\\"justification\\\":\\\"new justification\\\"}\"}}\n"
    );
    fs::write(&path, body).unwrap();

    let result = scan_codex_approval_updates(&path, 0, None).unwrap();
    assert_eq!(
        result.active_request,
        Some(CodexApprovalRequest {
            call_id: "call_new".into(),
            justification: "new justification".into(),
        })
    );

    fs::write(
        &path,
        format!(
            "{}{}",
            body,
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"call_new\",\"output\":\"done\"}}\n"
        ),
    )
    .unwrap();
    let result =
        scan_codex_approval_updates(&path, result.next_offset, result.active_request).unwrap();
    assert!(result.active_request.is_none());

    let _ = fs::remove_file(path);
}
#[test]
fn approval_callback_data_round_trips_request_id_and_choice() {
    let data = approval_callback_data("tg-123", "a");
    assert_eq!(data, "approval:tg-123:a");
    assert_eq!(parse_approval_callback_data(&data), Some(("tg-123", "a")));
    assert_eq!(parse_approval_callback_data("approval:y"), None);
}
#[test]
fn scan_codex_answer_updates_ignores_old_messages_before_offset() {
    let path = std::env::temp_dir().join(format!("pad-codex-answer-{}.jsonl", std::process::id()));
    let first = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"old\"}]}}\n";
    let second = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"final_answer\",\"content\":[{\"type\":\"output_text\",\"text\":\"new\"}]}}\n";
    fs::write(&path, format!("{first}{second}")).unwrap();

    let result =
        crate::chat::approval::scan_codex_answer_updates(&path, first.len() as u64, None).unwrap();
    assert_eq!(result.as_deref(), Some("new"));

    let _ = fs::remove_file(path);
}
#[test]
fn scan_codex_answer_updates_ignores_commentary_until_task_complete() {
    let path = std::env::temp_dir().join(format!(
        "pad-codex-answer-task-complete-{}.jsonl",
        std::process::id()
    ));
    let commentary = "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"intermediate\"}]}}\n";
    fs::write(&path, commentary).unwrap();

    let before_complete =
        crate::chat::approval::scan_codex_answer_updates(&path, 0, Some("turn-1")).unwrap();
    assert!(before_complete.is_none());

    let completed = concat!(
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"phase\":\"final_answer\",\"content\":[{\"type\":\"output_text\",\"text\":\"done\"}]}}\n",
        "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\",\"turn_id\":\"turn-1\",\"last_agent_message\":\"done\"}}\n"
    );
    fs::write(&path, format!("{commentary}{completed}")).unwrap();

    let after_complete =
        crate::chat::approval::scan_codex_answer_updates(&path, 0, Some("turn-1")).unwrap();
    assert_eq!(after_complete.as_deref(), Some("done"));

    let _ = fs::remove_file(path);
}
#[test]
fn scan_codex_failure_updates_detects_error_after_offset() {
    let path = std::env::temp_dir().join(format!("pad-codex-failure-{}.jsonl", std::process::id()));
    let old = "{\"type\":\"event_msg\",\"payload\":{\"type\":\"error\",\"message\":\"old failure\",\"codex_error_info\":\"other\"}}\n";
    let new = "{\"type\":\"event_msg\",\"payload\":{\"type\":\"error\",\"message\":\"unexpected status 502 Bad Gateway\",\"codex_error_info\":\"other\"}}\n";
    fs::write(&path, format!("{old}{new}")).unwrap();

    let result =
        crate::chat::approval::scan_codex_failure_updates(&path, old.len() as u64, None).unwrap();
    assert_eq!(
        result.failure.as_ref().map(|event| event.message.as_str()),
        Some("unexpected status 502 Bad Gateway")
    );
    assert_eq!(
        result
            .failure
            .as_ref()
            .and_then(|event| event.error_info.as_deref()),
        Some("other")
    );

    let _ = fs::remove_file(path);
}
#[test]
fn scan_codex_failure_updates_ignores_mismatched_turn_id() {
    let path = std::env::temp_dir().join(format!(
        "pad-codex-failure-turn-{}.jsonl",
        std::process::id()
    ));
    let body = "{\"type\":\"event_msg\",\"payload\":{\"type\":\"error\",\"turn_id\":\"turn-old\",\"message\":\"unexpected status 502 Bad Gateway\",\"codex_error_info\":\"other\"}}\n";
    fs::write(&path, body).unwrap();

    let result =
        crate::chat::approval::scan_codex_failure_updates(&path, 0, Some("turn-new")).unwrap();
    assert!(result.failure.is_none());

    let _ = fs::remove_file(path);
}
