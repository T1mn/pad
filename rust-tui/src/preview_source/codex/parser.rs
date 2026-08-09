#[path = "parser/function_call.rs"]
mod function_call {
    use super::model::TranscriptPayload;
    use serde_json::Value;

    pub(super) fn extract_spawn_agent_event_text_from_payload(
        payload: &TranscriptPayload<'_>,
    ) -> Option<String> {
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
}
#[path = "parser/lines.rs"]
mod lines {
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

    pub(super) fn parse_transcript_reader(
        mut reader: impl BufRead,
    ) -> io::Result<Vec<PreviewTurn>> {
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
}
#[path = "parser/message.rs"]
mod message {
    use super::model::TranscriptContent;
    use crate::preview_source::turns::SessionRole;

    pub(super) fn extract_message_text_from_items(
        content: &[TranscriptContent<'_>],
        role: SessionRole,
    ) -> (SessionRole, String) {
        if role == SessionRole::User {
            let text = extract_codex_user_message_text_from_items(content);
            if let Some(summary) =
                super::super::subagent::extract_subagent_notification_summary(&text)
            {
                return (SessionRole::Assistant, summary);
            }
            return (role, text);
        }

        (role, join_message_text_from_items(content, "output_text"))
    }

    fn join_message_text_from_items(
        content: &[TranscriptContent<'_>],
        target_type: &str,
    ) -> String {
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

        super::super::normalize_codex_user_text(&text, Some(image_count))
    }

    fn push_joined_part(out: &mut String, part: &str) {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(part);
    }
}
#[path = "parser/model.rs"]
mod model {
    use serde::Deserialize;
    use std::borrow::Cow;

    #[derive(Deserialize)]
    pub(super) struct TranscriptLine<'a> {
        #[serde(rename = "type", borrow)]
        pub(super) event_type: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub(super) payload: Option<TranscriptPayload<'a>>,
    }

    #[derive(Deserialize)]
    pub(super) struct TranscriptPayload<'a> {
        #[serde(rename = "type", borrow)]
        pub(super) kind: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub(super) role: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub(super) name: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub(super) arguments: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub(super) content: Option<Vec<TranscriptContent<'a>>>,
    }

    #[derive(Deserialize)]
    pub(super) struct TranscriptContent<'a> {
        #[serde(rename = "type", borrow)]
        pub(super) kind: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub(super) text: Option<Cow<'a, str>>,
    }
}

use super::tail;
use crate::model::PreviewTurn;
use crate::preview_source::SessionReadMode;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

pub(super) fn parse_transcript(
    path: &Path,
    _read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    parse_recent_transcript(path).map_err(|err| err.to_string())
}

fn parse_recent_transcript(path: &Path) -> std::io::Result<Vec<PreviewTurn>> {
    let rollout_path = resolve_rollout_path(path)?;
    if crate::codex_rollout::is_compressed_rollout(&rollout_path) {
        return parse_compressed_transcript(&rollout_path);
    }

    parse_plain_transcript(&rollout_path)
}

fn parse_plain_transcript(path: &Path) -> io::Result<Vec<PreviewTurn>> {
    let file_len = tail::file_len(path)?;
    if file_len == 0 {
        return Ok(Vec::new());
    }

    let mut tail_bytes = tail::initial_tail_bytes(file_len);
    loop {
        let lines = tail::read_tail_lines(path, file_len, tail_bytes)?;
        let turns = lines::parse_transcript_lines(lines.iter().map(String::as_str));
        if turns.len() >= crate::session_cache::SESSION_HISTORY_TURN_LIMIT || tail_bytes >= file_len
        {
            return Ok(turns);
        }
        tail_bytes = tail::grow_tail_bytes(tail_bytes, file_len);
    }
}

fn parse_compressed_transcript(path: &Path) -> io::Result<Vec<PreviewTurn>> {
    let file = File::open(path)?;
    let decoder = zstd::stream::read::Decoder::new(file)?;
    lines::parse_transcript_reader(BufReader::new(decoder))
}

fn resolve_rollout_path(path: &Path) -> io::Result<PathBuf> {
    crate::codex_rollout::existing_rollout_path(path).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("rollout file not found: {}", path.display()),
        )
    })
}
