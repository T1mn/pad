use super::model::OpenCodeThreadRef;
use super::util::{default_db_paths, open_readonly, to_io_error};
use rusqlite::OptionalExtension;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn all_threads(archived: Option<bool>) -> io::Result<Vec<OpenCodeThreadRef>> {
    let mut threads = Vec::new();
    for db_path in default_db_paths().into_iter().filter(|path| path.exists()) {
        threads.extend(query_threads_at(&db_path, archived)?);
    }
    threads.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    Ok(threads)
}

pub(crate) fn thread_for_id(session_id: &str) -> io::Result<Option<OpenCodeThreadRef>> {
    for db_path in default_db_paths().into_iter().filter(|path| path.exists()) {
        if let Some(thread) = query_thread_for_id_at(&db_path, session_id)? {
            return Ok(Some(thread));
        }
    }
    Ok(None)
}

pub(crate) fn threads_for_cwd(cwd: &Path) -> io::Result<Vec<OpenCodeThreadRef>> {
    let cwd = normalize_path(cwd).to_string_lossy().to_string();
    Ok(all_threads(Some(false))?
        .into_iter()
        .filter(|thread| normalize_path(&thread.cwd).to_string_lossy() == cwd)
        .collect())
}

pub(crate) fn db_path_for_session(session_id: &str) -> io::Result<Option<PathBuf>> {
    Ok(thread_for_id(session_id)?.map(|thread| thread.db_path))
}

pub(crate) fn query_threads_at(
    db_path: &Path,
    archived: Option<bool>,
) -> io::Result<Vec<OpenCodeThreadRef>> {
    let connection = open_readonly(db_path)?;
    if !has_table(&connection, "session")? || !has_table(&connection, "message")? {
        return Ok(Vec::new());
    }

    let where_clause = match archived {
        Some(true) => "WHERE time_archived IS NOT NULL",
        Some(false) => "WHERE time_archived IS NULL",
        None => "",
    };
    let sql = format!(
        "SELECT id, directory, path, title, time_updated, time_archived, model FROM session {where_clause} ORDER BY time_updated DESC"
    );
    let mut statement = connection.prepare(&sql).map_err(to_io_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                directory: row.get(1)?,
                path: row.get(2)?,
                title: row.get(3)?,
                updated_at: row.get(4)?,
                archived_at: row.get(5)?,
                model: row.get(6)?,
            })
        })
        .map_err(to_io_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_io_error)?;

    let session_ids = rows
        .iter()
        .map(|row| row.id.as_str())
        .collect::<HashSet<_>>();
    let summaries = load_message_summaries(&connection, &session_ids)?;
    Ok(rows
        .into_iter()
        .filter_map(|row| build_thread(db_path, &row, summaries.get(&row.id)))
        .collect())
}

fn query_thread_for_id_at(
    db_path: &Path,
    session_id: &str,
) -> io::Result<Option<OpenCodeThreadRef>> {
    let connection = open_readonly(db_path)?;
    if !has_table(&connection, "session")? || !has_table(&connection, "message")? {
        return Ok(None);
    }
    let row = connection
        .query_row(
            "SELECT id, directory, path, title, time_updated, time_archived, model FROM session WHERE id = ?1 LIMIT 1",
            [session_id],
            |row| {
                Ok(SessionRow {
                    id: row.get(0)?,
                    directory: row.get(1)?,
                    path: row.get(2)?,
                    title: row.get(3)?,
                    updated_at: row.get(4)?,
                    archived_at: row.get(5)?,
                    model: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(to_io_error)?;
    let Some(row) = row else { return Ok(None) };
    let summaries = load_message_summaries_for_session(&connection, session_id)?;
    Ok(build_thread(db_path, &row, summaries.get(session_id)))
}

#[derive(Clone)]
struct SessionRow {
    id: String,
    directory: String,
    path: Option<String>,
    title: Option<String>,
    updated_at: i64,
    archived_at: Option<i64>,
    model: Option<String>,
}

#[derive(Default)]
struct MessageSummary {
    first_user: Option<String>,
    last_user: Option<String>,
    last_assistant: Option<String>,
}

fn build_thread(
    db_path: &Path,
    row: &SessionRow,
    summary: Option<&MessageSummary>,
) -> Option<OpenCodeThreadRef> {
    let cwd = if row.directory.trim().is_empty() {
        row.path.as_deref().unwrap_or("")
    } else {
        row.directory.as_str()
    };
    if cwd.trim().is_empty() {
        return None;
    }
    let (provider_name, model_name) = parse_model(&row.model);
    Some(OpenCodeThreadRef {
        session_id: row.id.clone(),
        cwd: PathBuf::from(cwd),
        updated_at: row.updated_at,
        db_path: db_path.to_path_buf(),
        title: row.title.clone().filter(|title| !title.trim().is_empty()),
        first_user_message: summary.and_then(|summary| summary.first_user.clone()),
        last_user_message: summary.and_then(|summary| summary.last_user.clone()),
        last_assistant_message: summary.and_then(|summary| summary.last_assistant.clone()),
        provider_name,
        model_name,
        archived: row.archived_at.is_some(),
    })
}

fn load_message_summaries(
    connection: &rusqlite::Connection,
    session_ids: &HashSet<&str>,
) -> io::Result<HashMap<String, MessageSummary>> {
    let mut statement = connection
        .prepare(
            "SELECT m.session_id, m.data, p.data
             FROM message m
             LEFT JOIN part p ON p.message_id = m.id
             ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC",
        )
        .map_err(to_io_error)?;
    let mut rows = statement.query([]).map_err(to_io_error)?;
    let mut summaries: HashMap<String, MessageSummary> = HashMap::new();
    while let Some(row) = rows.next().map_err(to_io_error)? {
        let session_id: String = row.get(0).map_err(to_io_error)?;
        let message_data: String = row.get(1).map_err(to_io_error)?;
        let part_data: Option<String> = row.get(2).map_err(to_io_error)?;
        if session_ids.contains(session_id.as_str()) {
            apply_part(
                &mut summaries,
                session_id,
                &message_data,
                part_data.as_deref(),
            );
        }
    }
    Ok(summaries)
}

fn load_message_summaries_for_session(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> io::Result<HashMap<String, MessageSummary>> {
    let mut statement = connection
        .prepare(
            "SELECT m.session_id, m.data, p.data
             FROM message m
             LEFT JOIN part p ON p.message_id = m.id
             WHERE m.session_id = ?1
             ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC",
        )
        .map_err(to_io_error)?;
    let mut rows = statement.query([session_id]).map_err(to_io_error)?;
    let mut summaries: HashMap<String, MessageSummary> = HashMap::new();
    while let Some(row) = rows.next().map_err(to_io_error)? {
        let session_id: String = row.get(0).map_err(to_io_error)?;
        let message_data: String = row.get(1).map_err(to_io_error)?;
        let part_data: Option<String> = row.get(2).map_err(to_io_error)?;
        apply_part(
            &mut summaries,
            session_id,
            &message_data,
            part_data.as_deref(),
        );
    }
    Ok(summaries)
}

fn apply_part(
    summaries: &mut HashMap<String, MessageSummary>,
    session_id: String,
    message_data: &str,
    part_data: Option<&str>,
) {
    let role = serde_json::from_str::<Value>(message_data)
        .ok()
        .and_then(|value| {
            value
                .get("role")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    let Some(role) = role else { return };
    let text = part_data.and_then(extract_part_text).unwrap_or_default();
    if text.trim().is_empty() {
        return;
    }
    let summary = summaries.entry(session_id).or_default();
    match role.as_str() {
        "user" => {
            if summary.first_user.is_none() {
                summary.first_user = Some(text.clone());
            }
            summary.last_user = Some(text);
        }
        "assistant" => summary.last_assistant = Some(text),
        _ => {}
    }
}

fn extract_part_text(raw: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(raw).ok()?;
    extract_text_value(&value)
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

fn parse_model(raw: &Option<String>) -> (Option<String>, Option<String>) {
    let Some(raw) = raw.as_deref() else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return (None, None);
    };
    let provider = value
        .get("providerID")
        .or_else(|| value.get("provider"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let model = value
        .get("modelID")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    (provider, model)
}

fn has_table(connection: &rusqlite::Connection, table: &str) -> io::Result<bool> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            [table],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(to_io_error)
}

fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pad-opencode-{name}-{stamp}.db"))
    }

    fn seed_db(path: &Path) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE session (
                    id text PRIMARY KEY,
                    directory text NOT NULL,
                    path text,
                    title text NOT NULL,
                    time_updated integer NOT NULL,
                    time_archived integer,
                    model text
                );
                CREATE TABLE message (
                    id text PRIMARY KEY,
                    session_id text NOT NULL,
                    time_created integer NOT NULL,
                    data text NOT NULL
                );
                CREATE TABLE part (
                    id text PRIMARY KEY,
                    message_id text NOT NULL,
                    session_id text NOT NULL,
                    time_created integer NOT NULL,
                    data text NOT NULL
                );
                "#,
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO session (id, directory, path, title, time_updated, time_archived, model) VALUES (?1, ?2, NULL, ?3, ?4, NULL, ?5)",
                params![
                    "ses_1",
                    "/repo",
                    "Build feature",
                    100_i64,
                    r#"{"providerID":"wzw","id":"gpt-5.4"}"#
                ],
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
                params!["prt_1", "msg_1", "ses_1", 2_i64, r#"{"type":"text","text":"hello"}"#],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
                params!["msg_2", "ses_1", 3_i64, r#"{"role":"assistant"}"#],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO part (id, message_id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["prt_2", "msg_2", "ses_1", 4_i64, r#"{"type":"text","text":"world"}"#],
            )
            .unwrap();
    }

    #[test]
    fn query_threads_reads_opencode_sqlite() {
        let path = temp_db_path("query");
        seed_db(&path);

        let threads = query_threads_at(&path, Some(false)).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].session_id, "ses_1");
        assert_eq!(threads[0].last_user_message.as_deref(), Some("hello"));
        assert_eq!(threads[0].last_assistant_message.as_deref(), Some("world"));
        assert_eq!(threads[0].provider_name.as_deref(), Some("wzw"));
    }
}
