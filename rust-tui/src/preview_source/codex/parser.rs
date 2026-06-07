use super::normalize_codex_user_text;
use super::subagent::extract_subagent_notification_summary;
use super::tail;
use crate::model::PreviewTurn;
use crate::preview_source::turns::{finalize_turns, push_session_message, SessionRole};
use crate::preview_source::SessionReadMode;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::path::Path;

#[derive(Deserialize)]
struct TranscriptLine<'a> {
    #[serde(rename = "type", borrow)]
    event_type: Option<Cow<'a, str>>,
    #[serde(borrow)]
    payload: Option<TranscriptPayload<'a>>,
}

#[derive(Deserialize)]
struct TranscriptPayload<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<Cow<'a, str>>,
    #[serde(borrow)]
    role: Option<Cow<'a, str>>,
    #[serde(borrow)]
    name: Option<Cow<'a, str>>,
    #[serde(borrow)]
    arguments: Option<Cow<'a, str>>,
    #[serde(borrow)]
    content: Option<Vec<TranscriptContent<'a>>>,
}

#[derive(Deserialize)]
struct TranscriptContent<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<Cow<'a, str>>,
    #[serde(borrow)]
    text: Option<Cow<'a, str>>,
}

pub(super) fn parse_transcript(
    path: &Path,
    _read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    parse_recent_transcript(path).map_err(|err| err.to_string())
}

fn parse_recent_transcript(path: &Path) -> std::io::Result<Vec<PreviewTurn>> {
    let file_len = tail::file_len(path)?;
    if file_len == 0 {
        return Ok(Vec::new());
    }

    let mut tail_bytes = tail::initial_tail_bytes(file_len);
    loop {
        let lines = tail::read_tail_lines(path, file_len, tail_bytes)?;
        let turns = parse_transcript_lines(lines.iter().map(String::as_str));
        if turns.len() >= crate::session_cache::SESSION_HISTORY_TURN_LIMIT || tail_bytes >= file_len
        {
            return Ok(turns);
        }
        tail_bytes = tail::grow_tail_bytes(tail_bytes, file_len);
    }
}

fn parse_transcript_lines<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<PreviewTurn> {
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

fn extract_message_text_from_items(
    content: &[TranscriptContent<'_>],
    role: SessionRole,
) -> (SessionRole, String) {
    if role == SessionRole::User {
        let text = extract_codex_user_message_text_from_items(content);
        if let Some(summary) = extract_subagent_notification_summary(&text) {
            return (SessionRole::Assistant, summary);
        }
        return (role, text);
    }

    (role, join_message_text_from_items(content, "output_text"))
}

fn join_message_text_from_items(content: &[TranscriptContent<'_>], target_type: &str) -> String {
    let mut out = String::new();
    for item in content {
        if item.kind.as_deref() == Some(target_type) {
            if let Some(text) = item
                .text
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
            {
                push_joined_part(&mut out, text);
            }
        }
    }
    out
}

fn extract_codex_user_message_text_from_items(content: &[TranscriptContent<'_>]) -> String {
    let mut image_count = 0usize;
    let mut text = String::new();

    for item in content {
        match item.kind.as_deref() {
            Some("input_image") => image_count += 1,
            Some("input_text") => {
                if let Some(part) = item
                    .text
                    .as_deref()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                {
                    push_joined_part(&mut text, part);
                }
            }
            _ => {}
        }
    }

    normalize_codex_user_text(&text, Some(image_count))
}

fn push_joined_part(out: &mut String, part: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(part);
}

fn extract_spawn_agent_event_text_from_payload(payload: &TranscriptPayload<'_>) -> Option<String> {
    if payload.name.as_deref() != Some("spawn_agent") {
        return None;
    }

    let arguments = payload
        .arguments
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok());

    let task_name = arguments
        .as_ref()
        .and_then(|value| value.get("task_name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let agent_type = arguments
        .as_ref()
        .and_then(|value| value.get("agent_type"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let kind = agent_type.unwrap_or("worker");
    let task = task_name.unwrap_or("task");
    Some(format!("[subagent/start][{}] {}", kind, task))
}
