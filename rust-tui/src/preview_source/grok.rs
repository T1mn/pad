use super::turns::finalize_turns;
use super::SessionReadMode;
use crate::model::PreviewTurn;
use serde_json::Value;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::parse_reader;
    use std::io::Cursor;

    #[test]
    fn parses_official_0_2_102_envelopes_and_skips_unknown_lines() {
        let input = concat!(
            "{not-json\n",
            r#"{"timestamp":1,"method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"hello"}}}}"#,
            "\n",
            r#"{"timestamp":2,"method":"_x.ai/session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"hi "}}}}"#,
            "\n",
            r#"{"timestamp":3,"method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"there"},"future":true}}}"#,
            "\n",
            r#"{"timestamp":4,"method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"tool_call","title":"ignored"}}}"#,
            "\n",
        );

        let turns = parse_reader(Cursor::new(input)).unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "hello");
        assert_eq!(turns[0].answer.as_deref(), Some("hi there"));
    }

    #[test]
    fn accepts_direct_update_shape_for_older_logs() {
        let input = concat!(
            r#"{"update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"q"}}}"#,
            "\n",
            r#"{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"a"}}}"#,
            "\n",
        );
        let turns = parse_reader(Cursor::new(input)).unwrap();
        assert_eq!(turns[0].question, "q");
        assert_eq!(turns[0].answer.as_deref(), Some("a"));
    }
}
