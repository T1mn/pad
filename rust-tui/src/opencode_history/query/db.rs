use super::super::model::OpenCodeThreadRef;
use super::super::stats::{read_session_stats, session_stats_select};
use super::super::util::{open_readonly, to_io_error};
use super::messages::{load_message_summaries, load_message_summaries_for_session};
use super::thread::{build_thread, SessionRow};
use rusqlite::OptionalExtension;
use std::collections::HashSet;
use std::io;
use std::path::Path;

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
    let stats_select = session_stats_select(&connection)?;
    let sql = format!(
        "SELECT id, directory, path, title, time_updated, time_archived, model, {stats_select} FROM session {where_clause} ORDER BY time_updated DESC"
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
                stats: read_session_stats(row, 7)?,
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

pub(super) fn query_thread_for_id_at(
    db_path: &Path,
    session_id: &str,
) -> io::Result<Option<OpenCodeThreadRef>> {
    let connection = open_readonly(db_path)?;
    if !has_table(&connection, "session")? || !has_table(&connection, "message")? {
        return Ok(None);
    }
    let stats_select = session_stats_select(&connection)?;
    let sql = format!(
        "SELECT id, directory, path, title, time_updated, time_archived, model, {stats_select} FROM session WHERE id = ?1 LIMIT 1"
    );
    let row = connection
        .query_row(&sql, [session_id], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                directory: row.get(1)?,
                path: row.get(2)?,
                title: row.get(3)?,
                updated_at: row.get(4)?,
                archived_at: row.get(5)?,
                model: row.get(6)?,
                stats: read_session_stats(row, 7)?,
            })
        })
        .optional()
        .map_err(to_io_error)?;
    let Some(row) = row else { return Ok(None) };
    let summaries = load_message_summaries_for_session(&connection, session_id)?;
    Ok(build_thread(db_path, &row, summaries.get(session_id)))
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
