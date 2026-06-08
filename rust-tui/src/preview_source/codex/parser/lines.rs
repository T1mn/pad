use super::function_call::extract_spawn_agent_event_text_from_payload;
use super::message::extract_message_text_from_items;
use super::model::TranscriptLine;
use crate::model::PreviewTurn;
use crate::preview_source::turns::{finalize_turns, push_session_message, SessionRole};
use std::collections::VecDeque;

pub(super) fn parse_transcript_lines<'a>(
    lines: impl IntoIterator<Item = &'a str>,
) -> Vec<PreviewTurn> {
    let mut turns = VecDeque::new();

    for line in lines {
        let Ok(value) = serde_json::from_str::<TranscriptLine<'_>>(line) else {
            continue;
        };

        if value.event_type.as_deref() != Some("response_item") {
            continue;
        }

        let payload = match value.payload {
            Some(payload) => payload,
            None => continue,
        };

        match payload.kind.as_deref() {
            Some("message") => {
                let role = match payload.role.as_deref() {
                    Some("user") => SessionRole::User,
                    Some("assistant") => SessionRole::Assistant,
                    _ => continue,
                };

                let content = payload.content.as_deref().unwrap_or(&[]);
                let (effective_role, text) = extract_message_text_from_items(content, role);
                push_session_message(&mut turns, effective_role, text);
            }
            Some("function_call") => {
                if let Some(summary) = extract_spawn_agent_event_text_from_payload(&payload) {
                    push_session_message(&mut turns, SessionRole::Assistant, summary);
                }
            }
            _ => {}
        }
    }

    finalize_turns(turns)
}
