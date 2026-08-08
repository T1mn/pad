mod message {
    use serde_json::Value;

    pub(super) fn parse_message_text(value: &Value) -> Option<String> {
        match value {
            Value::String(text) => clean_text(text),
            Value::Array(values) => parse_message_text_array(values),
            Value::Object(map) => parse_message_text_object(map),
            _ => None,
        }
    }

    fn parse_message_text_array(values: &[Value]) -> Option<String> {
        let mut joined = String::new();
        for part in values.iter().filter_map(parse_message_text) {
            if part.trim().is_empty() {
                continue;
            }
            if !joined.is_empty() {
                joined.push('\n');
            }
            joined.push_str(&part);
        }
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    }

    fn parse_message_text_object(map: &serde_json::Map<String, Value>) -> Option<String> {
        ["text", "content", "parts"]
            .iter()
            .find_map(|key| map.get(*key).and_then(parse_message_text))
    }

    fn clean_text(text: &str) -> Option<String> {
        let cleaned = text.trim();
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned.to_string())
        }
    }
}
mod project {
    use crate::gemini_history::util::normalize_path;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    pub(super) fn project_root_for_session_path(path: &Path) -> io::Result<(PathBuf, String)> {
        let project_dir = path
            .parent()
            .and_then(|parent| parent.parent())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid Gemini session path: {}", path.display()),
                )
            })?;
        let project_alias = project_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("unknown"));
        let project_root =
            read_project_root_file(project_dir).unwrap_or_else(|| normalize_path(project_dir));
        Ok((project_root, project_alias))
    }

    fn read_project_root_file(project_dir: &Path) -> Option<PathBuf> {
        fs::read_to_string(project_dir.join(".project_root"))
            .ok()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
            .map(PathBuf::from)
            .map(|path| normalize_path(&path))
    }
}
mod snapshot {
    use super::message::parse_message_text;
    use super::project::project_root_for_session_path;
    use crate::gemini_history::model::GeminiSnapshot;
    use crate::gemini_history::util::{file_mtime_secs, md5_hex, parse_timestamp, read_text};
    use serde_json::Value;
    use std::io;
    use std::path::Path;

    pub(in crate::gemini_history::scan) fn parse_snapshot(
        path: &Path,
    ) -> io::Result<Option<GeminiSnapshot>> {
        let text = read_text(path)?;
        let value: Value = serde_json::from_str(&text).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse {}: {}", path.display(), err),
            )
        })?;

        let Some(session_id) = value.get("sessionId").and_then(Value::as_str) else {
            return Ok(None);
        };
        let Some(kind) = value.get("kind").and_then(Value::as_str) else {
            return Ok(None);
        };

        let (project_root, project_alias) = project_root_for_session_path(path)?;
        let start_time = value
            .get("startTime")
            .and_then(Value::as_str)
            .and_then(parse_timestamp)
            .or_else(|| file_mtime_secs(path))
            .unwrap_or_default();
        let last_updated = value
            .get("lastUpdated")
            .and_then(Value::as_str)
            .and_then(parse_timestamp)
            .or_else(|| file_mtime_secs(path))
            .unwrap_or(start_time);
        let summary = value
            .get("summary")
            .and_then(parse_message_text)
            .or_else(|| {
                value
                    .get("summary")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            });
        let messages = extract_message_texts(&value);

        Ok(Some(GeminiSnapshot {
            session_id: session_id.to_string(),
            project_root,
            project_alias,
            transcript_path: path.to_path_buf(),
            kind: kind.to_string(),
            start_time,
            last_updated,
            summary,
            first_user_message: messages.first_user_message,
            last_user_message: messages.last_user_message,
            last_assistant_message: messages.last_assistant_message,
            payload_hash: md5_hex(&text),
        }))
    }

    fn extract_message_texts(value: &Value) -> ParsedMessages {
        let mut messages = ParsedMessages::default();
        let Some(raw_messages) = value.get("messages").and_then(Value::as_array) else {
            return messages;
        };

        for message in raw_messages {
            let Some(message_type) = message.get("type").and_then(Value::as_str) else {
                continue;
            };
            let content = message.get("content").and_then(parse_message_text);
            if content.as_deref().map(str::trim).unwrap_or("").is_empty() {
                continue;
            }

            match message_type {
                "user" => {
                    if messages.first_user_message.is_none() {
                        messages.first_user_message = content.clone();
                    }
                    messages.last_user_message = content;
                }
                "gemini" => messages.last_assistant_message = content,
                _ => {}
            }
        }

        messages
    }

    #[derive(Default)]
    struct ParsedMessages {
        first_user_message: Option<String>,
        last_user_message: Option<String>,
        last_assistant_message: Option<String>,
    }
}

pub(super) use snapshot::parse_snapshot;
