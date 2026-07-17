use super::function_call::extract_spawn_agent_event_text_from_payload;
use super::message::extract_message_text_from_items;
use super::model::TranscriptLine;
use crate::model::PreviewTurn;
use crate::preview_source::turns::{finalize_turns, push_session_message, SessionRole};
use std::collections::VecDeque;
use std::io::{self, BufRead};

pub(super) fn parse_transcript_lines<'a>(
    lines: impl IntoIterator<Item = &'a str>,
) -> Vec<PreviewTurn> {
    let mut turns = VecDeque::new();

    for line in lines {
        parse_transcript_line(&mut turns, line);
    }

    finalize_turns(turns)
}

pub(super) fn parse_transcript_reader(mut reader: impl BufRead) -> io::Result<Vec<PreviewTurn>> {
    let mut turns = VecDeque::new();
    let mut line = Vec::new();

    while reader.read_until(b'\n', &mut line)? > 0 {
        parse_transcript_line(&mut turns, &String::from_utf8_lossy(&line));
        line.clear();
    }

    Ok(finalize_turns(turns))
}

fn parse_transcript_line(turns: &mut VecDeque<PreviewTurn>, line: &str) {
    let Ok(value) = serde_json::from_str::<TranscriptLine<'_>>(line) else {
        return;
    };

    if value.event_type.as_deref() != Some("response_item") {
        return;
    }

    let payload = match value.payload {
        Some(payload) => payload,
        None => return,
    };

    match payload.kind.as_deref() {
        Some("message") => {
            let role = match payload.role.as_deref() {
                Some("user") => SessionRole::User,
                Some("assistant") => SessionRole::Assistant,
                _ => return,
            };

            let content = payload.content.as_deref().unwrap_or(&[]);
            let (effective_role, text) = extract_message_text_from_items(content, role);
            push_session_message(turns, effective_role, text);
        }
        Some("function_call") => {
            if let Some(summary) = extract_spawn_agent_event_text_from_payload(&payload) {
                push_session_message(turns, SessionRole::Assistant, summary);
            }
        }
        _ => {}
    }
}
