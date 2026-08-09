use super::SessionReadMode;
use crate::model::PreviewTurn;
use std::path::Path;

mod message {
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
}
mod read {
    use serde_json::Value;
    use std::fs::File;
    use std::io::{BufReader, Read};
    use std::path::Path;

    pub(super) fn read_transcript_value(path: &Path) -> Result<Value, String> {
        let file = File::open(path).map_err(|err| err.to_string())?;
        let mut reader = BufReader::new(file);
        let mut text = String::new();
        reader
            .read_to_string(&mut text)
            .map_err(|err| err.to_string())?;

        serde_json::from_str(&text).map_err(|err| err.to_string())
    }
}
mod text {
    use serde_json::Value;

    pub(super) fn extract_message_text(value: &Value) -> String {
        match value {
            Value::String(text) => text.trim().to_string(),
            Value::Array(items) => extract_array_text(items),
            Value::Object(map) => extract_object_text(map),
            _ => String::new(),
        }
    }

    fn extract_array_text(items: &[Value]) -> String {
        let mut joined = String::new();
        for item in items {
            let text = extract_message_text(item);
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            if !joined.is_empty() {
                joined.push('\n');
            }
            joined.push_str(text);
        }
        joined
    }

    fn extract_object_text(map: &serde_json::Map<String, Value>) -> String {
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
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
pub(crate) mod tests {
    use super::parse_transcript;
    use crate::preview_source::SessionReadMode;
    use std::fs;

    fn temp_json_path(name: &str) -> std::path::PathBuf {
        crate::test_support::temp_path("pad-preview-json", name)
    }

    pub(crate) fn parse_gemini_transcript_skips_info_and_keeps_pairs() {
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

    pub(crate) fn extract_session_id_from_transcript_reads_root_metadata() {
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

    pub(crate) fn parse_gemini_transcript_joins_nested_non_empty_text_parts() {
        let path = temp_json_path("gemini-nested-parts");
        fs::write(
            &path,
            concat!(
                "{",
                "\"messages\":[",
                "{\"type\":\"user\",\"content\":[{\"text\":\"hello\"},{\"text\":\"   \"},{\"content\":{\"text\":\"world\"}}]},",
                "{\"type\":\"gemini\",\"content\":[{\"text\":\"answer\"},{\"parts\":[{\"text\":\"more\"}]}]}",
                "]}"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "hello\nworld");
        assert_eq!(turns[0].answer.as_deref(), Some("answer\nmore"));
    }
}

pub(super) fn parse_transcript(
    path: &Path,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    match read_mode {
        SessionReadMode::FullBackfill => message::parse_full_transcript(path),
    }
}

pub(super) fn extract_session_id_from_transcript(path: &Path) -> Option<String> {
    read::read_transcript_value(path)
        .ok()
        .and_then(|value| {
            value
                .get("sessionId")
                .and_then(serde_json::Value::as_str)
                .map(|session_id| session_id.trim().to_string())
        })
        .filter(|session_id| !session_id.is_empty())
}
