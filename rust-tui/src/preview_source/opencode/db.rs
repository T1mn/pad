use super::text::{extract_part_text, message_role};
use crate::model::PreviewTurn;
use crate::preview_source::turns::{finalize_turns, push_session_message};
use rusqlite::OptionalExtension;
use std::collections::VecDeque;
use std::path::Path;

pub(super) fn parse_session(db_path: &Path, session_id: &str) -> Result<Vec<PreviewTurn>, String> {
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
