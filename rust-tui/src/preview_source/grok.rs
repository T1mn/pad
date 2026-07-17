use super::turns::finalize_turns;
use super::SessionReadMode;
use crate::model::PreviewTurn;
use serde_json::Value;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[cfg(test)]
mod tests;

pub(super) fn parse_transcript(
    transcript_path: &Path,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    match read_mode {
        SessionReadMode::FullBackfill => parse_reader(BufReader::new(
            File::open(transcript_path).map_err(|err| err.to_string())?,
        )),
    }
}

fn parse_reader(reader: impl BufRead) -> Result<Vec<PreviewTurn>, String> {
    let mut turns = VecDeque::new();
    let mut last_role = None;
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let update = value
            .pointer("/params/update")
            .or_else(|| value.get("update"))
            .unwrap_or(&value);
        let Some(kind) = update.get("sessionUpdate").and_then(Value::as_str) else {
            continue;
        };
        let Some(text) = update.pointer("/content/text").and_then(Value::as_str) else {
            continue;
        };
        append_chunk(&mut turns, &mut last_role, kind, text);
    }
    Ok(finalize_turns(turns))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChunkRole {
    User,
    Assistant,
}

fn append_chunk(
    turns: &mut VecDeque<PreviewTurn>,
    last_role: &mut Option<ChunkRole>,
    kind: &str,
    text: &str,
) {
    let role = match kind {
        "user_message_chunk" => ChunkRole::User,
        "agent_message_chunk" => ChunkRole::Assistant,
        _ => return,
    };
    if text.is_empty() {
        return;
    }

    match role {
        ChunkRole::User if *last_role == Some(ChunkRole::User) => {
            if let Some(turn) = turns.back_mut() {
                turn.question.push_str(text);
            }
        }
        ChunkRole::User => {
            turns.push_back(PreviewTurn {
                question: text.to_string(),
                answer: None,
            });
            while turns.len() > crate::session_cache::SESSION_HISTORY_TURN_LIMIT {
                turns.pop_front();
            }
        }
        ChunkRole::Assistant => {
            if let Some(turn) = turns.back_mut() {
                turn.answer.get_or_insert_with(String::new).push_str(text);
            }
        }
    }
    *last_role = Some(role);
}
