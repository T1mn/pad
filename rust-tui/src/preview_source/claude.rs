mod line {
    use super::super::SessionReadMode;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    pub(super) fn for_each_session_line<F>(
        path: &Path,
        read_mode: SessionReadMode,
        mut f: F,
    ) -> std::io::Result<()>
    where
        F: FnMut(&str),
    {
        match read_mode {
            SessionReadMode::FullBackfill => {
                let file = File::open(path)?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    f(&line?);
                }
            }
        }

        Ok(())
    }
}
mod text {
    use super::super::turns::SessionRole;
    use serde_json::Value;

    pub(super) fn message_text(value: &Value) -> Option<(SessionRole, String)> {
        if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
            return None;
        }

        let role = match value.get("type").and_then(Value::as_str)? {
            "user" => SessionRole::User,
            "assistant" => SessionRole::Assistant,
            _ => return None,
        };
        let message = value.get("message")?;
        let text = match role {
            SessionRole::User => extract_user_text(message),
            SessionRole::Assistant => extract_assistant_text(message),
        };

        Some((role, text))
    }

    fn extract_user_text(message: &Value) -> String {
        if message.get("role").and_then(Value::as_str) != Some("user") {
            return String::new();
        }

        match message.get("content") {
            Some(Value::String(text)) => sanitize_user_string(text),
            Some(Value::Array(items)) => join_non_empty_text(items.iter().filter_map(|item| {
                if item.get("type").and_then(Value::as_str) != Some("text") {
                    return None;
                }
                item.get("text")
                    .and_then(Value::as_str)
                    .map(sanitize_user_string)
                    .filter(|text| !text.is_empty())
            })),
            _ => String::new(),
        }
    }

    fn sanitize_user_string(text: &str) -> String {
        let trimmed = text.trim();
        if trimmed.is_empty()
            || trimmed.contains("<command-name>")
            || trimmed.contains("<local-command")
        {
            return String::new();
        }

        trimmed.to_string()
    }

    fn extract_assistant_text(message: &Value) -> String {
        if message.get("role").and_then(Value::as_str) != Some("assistant") {
            return String::new();
        }

        match message.get("content") {
            Some(Value::String(text)) => text.trim().to_string(),
            Some(Value::Array(items)) => join_non_empty_text(items.iter().filter_map(|item| {
                if item.get("type").and_then(Value::as_str) != Some("text") {
                    return None;
                }
                item.get("text")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
            })),
            _ => String::new(),
        }
    }

    fn join_non_empty_text<'a>(items: impl Iterator<Item = impl AsRef<str> + 'a>) -> String {
        let mut joined = String::new();
        for item in items {
            if !joined.is_empty() {
                joined.push('\n');
            }
            joined.push_str(item.as_ref());
        }
        joined
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::parse_transcript;
    use crate::preview_source::SessionReadMode;
    use std::fs;

    fn temp_jsonl_path(name: &str) -> std::path::PathBuf {
        crate::test_support::temp_path("pad-preview-jsonl", name)
    }

    #[test]
    fn parse_claude_transcript_skips_meta_thinking_and_tools() {
        let path = temp_jsonl_path("claude");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"user\",\"isMeta\":true,\"message\":{\"role\":\"user\",\"content\":\"skip meta\"}}\n",
                "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"<command-name>/clear</command-name>\"}}\n",
                "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"real user\"}}\n",
                "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"skip\"},{\"type\":\"text\",\"text\":\"real assistant\"}]}}\n",
                "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"tool_result\",\"content\":\"skip tool\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "real user");
        assert_eq!(turns[0].answer.as_deref(), Some("real assistant"));
    }

    #[test]
    fn parse_claude_transcript_joins_text_array_parts() {
        let path = temp_jsonl_path("claude-multipart");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"first\"},{\"type\":\"tool_result\",\"content\":\"skip\"},{\"type\":\"text\",\"text\":\" second \"}]}}\n",
                "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"answer\"},{\"type\":\"thinking\",\"thinking\":\"skip\"},{\"type\":\"text\",\"text\":\"more\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "first\nsecond");
        assert_eq!(turns[0].answer.as_deref(), Some("answer\nmore"));
    }
}

use super::turns::{finalize_turns, push_session_message};
use super::SessionReadMode;
use crate::model::PreviewTurn;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::Path;

pub(super) fn parse_transcript(
    path: &Path,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    let mut turns = VecDeque::new();

    line::for_each_session_line(path, read_mode, |line| {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return;
        };
        let Some((role, text)) = text::message_text(&value) else {
            return;
        };
        push_session_message(&mut turns, role, text);
    })
    .map_err(|err| err.to_string())?;

    Ok(finalize_turns(turns))
}
