use super::codex_final_answer_line;

#[test]
fn final_answer_joins_output_text_blocks() {
    let payload = serde_json::json!({
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": "assistant",
            "phase": "final_answer",
            "content": [
                {"type": "output_text", "text": " first "},
                {"type": "thinking", "text": "ignored"},
                {"type": "output_text", "text": "second"}
            ]
        }
    });

    assert_eq!(
        codex_final_answer_line(&payload).as_deref(),
        Some("first\nsecond")
    );
}

#[test]
fn final_answer_ignores_empty_output_text_blocks() {
    let payload = serde_json::json!({
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": "assistant",
            "phase": "final_answer",
            "content": [
                {"type": "output_text", "text": "   "}
            ]
        }
    });

    assert_eq!(codex_final_answer_line(&payload), None);
}
