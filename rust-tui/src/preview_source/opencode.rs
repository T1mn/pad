use super::turns::{finalize_turns, push_session_message, SessionRole};
use super::SessionReadMode;
use crate::model::PreviewTurn;
use rusqlite::OptionalExtension;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::Path;

pub(super) fn parse_transcript(
    db_path: &Path,
    session_id: Option<&str>,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    let Some(session_id) = session_id else {
        return Ok(Vec::new());
    };

    match read_mode {
        SessionReadMode::FullBackfill => parse_session(db_path, session_id),
    }
}

fn parse_session(db_path: &Path, session_id: &str) -> Result<Vec<PreviewTurn>, String> {
    let connection = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| err.to_string())?;

    if !has_table(&connection, "message").map_err(|err| err.to_string())?
        || !has_table(&connection, "part").map_err(|err| err.to_string())?
    {
        return Ok(Vec::new());
    }

    let mut statement = connection
        .prepare(
            "SELECT m.data, p.data
             FROM message m
             LEFT JOIN part p ON p.message_id = m.id
             WHERE m.session_id = ?1
             ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC",
        )
        .map_err(|err| err.to_string())?;
    let mut rows = statement
        .query([session_id])
        .map_err(|err| err.to_string())?;
    let mut turns = VecDeque::new();
    while let Some(row) = rows.next().map_err(|err| err.to_string())? {
        let message_data: String = row.get(0).map_err(|err| err.to_string())?;
        let part_data: Option<String> = row.get(1).map_err(|err| err.to_string())?;
        let Some(role) = message_role(&message_data) else {
            continue;
        };
        let text = part_data
            .as_deref()
            .and_then(extract_part_text)
            .unwrap_or_default();
        push_session_message(&mut turns, role, text);
    }

    Ok(finalize_turns(turns))
}

fn message_role(raw: &str) -> Option<SessionRole> {
    let value = serde_json::from_str::<Value>(raw).ok()?;
    match value.get("role").and_then(Value::as_str)? {
        "user" => Some(SessionRole::User),
        "assistant" => Some(SessionRole::Assistant),
        _ => None,
    }
}

fn extract_part_text(raw: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(raw).ok()?;
    match value.get("type").and_then(Value::as_str) {
        Some("text") | Some("reasoning") | Some("step-start") | None => extract_text_value(&value),
        _ => None,
    }
}

fn extract_text_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => non_empty(text),
        Value::Array(items) => join_text(items.iter().filter_map(extract_text_value)),
        Value::Object(map) => {
            for key in ["text", "content", "message", "value"] {
                if let Some(text) = map.get(key).and_then(extract_text_value) {
                    return Some(text);
                }
            }
            None
        }
        _ => None,
    }
}

fn join_text(items: impl Iterator<Item = String>) -> Option<String> {
    let text = items.collect::<Vec<_>>().join("\n");
    non_empty(&text)
}

fn non_empty(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn has_table(connection: &rusqlite::Connection, table: &str) -> rusqlite::Result<bool> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            [table],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pad-opencode-preview-{stamp}.db"))
    }

    #[test]
    fn parses_opencode_sqlite_messages_into_turns() {
        let path = temp_db_path();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE message (id text PRIMARY KEY, session_id text NOT NULL, time_created integer NOT NULL, data text NOT NULL);
                CREATE TABLE part (id text PRIMARY KEY, message_id text NOT NULL, session_id text NOT NULL, time_created integer NOT NULL, data text NOT NULL);
                "#,
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
                params!["msg_1", "ses_1", 1_i64, r#"{"role":"user"}"#],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO part (id, message_id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["prt_1", "msg_1", "ses_1", 1_i64, r#"{"type":"text","text":"hello"}"#],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
                params!["msg_2", "ses_1", 2_i64, r#"{"role":"assistant"}"#],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO part (id, message_id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["prt_2", "msg_2", "ses_1", 2_i64, r#"{"type":"text","text":"world"}"#],
            )
            .unwrap();
        drop(connection);

        let turns = parse_transcript(&path, Some("ses_1"), SessionReadMode::FullBackfill).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "hello");
        assert_eq!(turns[0].answer.as_deref(), Some("world"));
    }
}
