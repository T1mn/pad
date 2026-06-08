use super::read::read_transcript_value;
use super::text::extract_message_text;
use crate::model::PreviewTurn;
use crate::preview_source::turns::{finalize_turns, push_session_message, SessionRole};
use serde_json::Value;
use std::collections::VecDeque;
use std::path::Path;

pub(super) fn parse_full_transcript(path: &Path) -> Result<Vec<PreviewTurn>, String> {
    let value = read_transcript_value(path)?;
    let Some(messages) = value.get("messages").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    let mut turns = VecDeque::new();
    for message in messages {
        let Some(role) = message_role(message) else {
            continue;
        };
        let Some(content) = message.get("content") else {
            continue;
        };
        push_session_message(&mut turns, role, extract_message_text(content));
    }
    Ok(finalize_turns(turns))
}

fn message_role(message: &Value) -> Option<SessionRole> {
    match message.get("type").and_then(Value::as_str)? {
        "user" => Some(SessionRole::User),
        "gemini" | "assistant" => Some(SessionRole::Assistant),
        _ => None,
    }
}
