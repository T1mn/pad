use super::turns::{finalize_turns, push_session_message, SessionRole};
use super::SessionReadMode;
use crate::model::PreviewTurn;
use serde_json::Value;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub(super) fn parse_transcript(
    path: &Path,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    let mut turns = VecDeque::new();

    match read_mode {
        SessionReadMode::FullBackfill => {
            let value = read_transcript_value(path)?;
            let Some(messages) = value.get("messages").and_then(Value::as_array) else {
                return Ok(Vec::new());
            };

            for message in messages {
                let Some(role) = message.get("type").and_then(Value::as_str) else {
                    continue;
                };

                let role = match role {
                    "user" => SessionRole::User,
                    "gemini" | "assistant" => SessionRole::Assistant,
                    _ => continue,
                };

                let Some(content) = message.get("content") else {
                    continue;
                };

                let text = extract_message_text(content);
                push_session_message(&mut turns, role, text);
            }
        }
    }

    Ok(finalize_turns(turns))
}

pub(super) fn extract_session_id_from_transcript(path: &Path) -> Option<String> {
    read_transcript_value(path)
        .ok()
        .and_then(|value| {
            value
                .get("sessionId")
                .and_then(Value::as_str)
                .map(|session_id| session_id.trim().to_string())
        })
        .filter(|session_id| !session_id.is_empty())
}

fn read_transcript_value(path: &Path) -> Result<Value, String> {
    let file = File::open(path).map_err(|err| err.to_string())?;
    let mut reader = BufReader::new(file);
    let mut text = String::new();
    reader
        .read_to_string(&mut text)
        .map_err(|err| err.to_string())?;

    serde_json::from_str(&text).map_err(|err| err.to_string())
}

fn extract_message_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.trim().to_string(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                let text = extract_message_text(item);
                let text = text.trim();
                if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                return text.trim().to_string();
            }
            if let Some(content) = map.get("content") {
                return extract_message_text(content);
            }
            if let Some(parts) = map.get("parts") {
                return extract_message_text(parts);
            }
            String::new()
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_transcript;
    use crate::preview_source::SessionReadMode;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_json_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pad-preview-{}-{}.json", name, stamp))
    }

    #[test]
    fn parse_gemini_transcript_skips_info_and_keeps_pairs() {
        let path = temp_json_path("gemini");
        fs::write(
            &path,
            concat!(
                "{",
                "\"sessionId\":\"sess-1\",",
                "\"kind\":\"main\",",
                "\"messages\":[",
                "{\"type\":\"info\",\"content\":\"skip me\"},",
                "{\"type\":\"user\",\"content\":[{\"text\":\"hello\"}]},",
                "{\"type\":\"gemini\",\"content\":\"world\"},",
                "{\"type\":\"user\",\"content\":{\"text\":\"second\"}},",
                "{\"type\":\"assistant\",\"content\":{\"parts\":[{\"text\":\"line 1\"},{\"text\":\"line 2\"}]}}",
                "]}"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].question, "second");
        assert_eq!(turns[0].answer.as_deref(), Some("line 1\nline 2"));
        assert_eq!(turns[1].question, "hello");
        assert_eq!(turns[1].answer.as_deref(), Some("world"));
    }

    #[test]
    fn extract_session_id_from_transcript_reads_root_metadata() {
        let path = temp_json_path("gemini-meta");
        fs::write(
            &path,
            concat!(
                "{",
                "\"sessionId\":\"sess-meta-1\",",
                "\"kind\":\"main\",",
                "\"messages\":[]",
                "}"
            ),
        )
        .unwrap();

        let session_id = super::extract_session_id_from_transcript(&path);
        fs::remove_file(&path).ok();

        assert_eq!(session_id.as_deref(), Some("sess-meta-1"));
    }
}
