use super::super::model::GeminiThreadRef;
use super::super::util::normalize_path;
use super::schema::{ensure_schema, open_index_db, to_io_error, DB_TABLE};
use rusqlite::OptionalExtension;
use std::io;
use std::path::{Path, PathBuf};

const SELECT_COLUMNS: &str = "session_id, cwd, updated_at, transcript_path, title, subtitle,
                    first_user_message, last_user_message, last_assistant_message,
                    summary, kind, archived, has_subagent";

pub(crate) fn query_threads(
    db_path: &Path,
    archived: Option<bool>,
) -> io::Result<Vec<GeminiThreadRef>> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let sql = select_sql(
        archive_where(archived),
        "updated_at DESC, session_id DESC, cwd DESC",
    );
    collect_rows(&connection, &sql, [])
}

pub(crate) fn query_thread_for_id(
    db_path: &Path,
    session_id: &str,
) -> io::Result<Option<GeminiThreadRef>> {
    if !db_path.exists() {
        return Ok(None);
    }

    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    connection
        .query_row(
            &format!(
                "SELECT {SELECT_COLUMNS}
                 FROM {DB_TABLE}
                 WHERE session_id = ?1
                 ORDER BY archived ASC, updated_at DESC, cwd DESC
                 LIMIT 1"
            ),
            [session_id],
            map_row,
        )
        .optional()
        .map_err(to_io_error)
}

pub(crate) fn query_threads_for_cwd(
    db_path: &Path,
    cwd: &Path,
    archived: Option<bool>,
) -> io::Result<Vec<GeminiThreadRef>> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let cwd_key = normalize_path(cwd).to_string_lossy().to_string();
    let sql = select_sql(
        cwd_archive_where(archived),
        "updated_at DESC, session_id DESC",
    );
    collect_rows(&connection, &sql, [cwd_key])
}

fn archive_where(archived: Option<bool>) -> &'static str {
    match archived {
        Some(true) => "WHERE archived = 1",
        Some(false) => "WHERE archived = 0",
        None => "",
    }
}

fn cwd_archive_where(archived: Option<bool>) -> &'static str {
    match archived {
        Some(true) => "WHERE cwd = ?1 AND archived = 1",
        Some(false) => "WHERE cwd = ?1 AND archived = 0",
        None => "WHERE cwd = ?1",
    }
}

fn select_sql(where_clause: &str, order_by: &str) -> String {
    if where_clause.is_empty() {
        format!(
            "SELECT {SELECT_COLUMNS}
             FROM {DB_TABLE}
             ORDER BY {order_by}"
        )
    } else {
        format!(
            "SELECT {SELECT_COLUMNS}
             FROM {DB_TABLE}
             {where_clause}
             ORDER BY {order_by}"
        )
    }
}

fn collect_rows<P: rusqlite::Params>(
    connection: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> io::Result<Vec<GeminiThreadRef>> {
    let mut statement = connection.prepare(sql).map_err(to_io_error)?;
    let rows = statement
        .query_map(params, map_row)
        .map_err(to_io_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_io_error)?;
    Ok(rows)
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GeminiThreadRef> {
    Ok(GeminiThreadRef {
        session_id: row.get(0)?,
        cwd: PathBuf::from(row.get::<_, String>(1)?),
        updated_at: row.get(2)?,
        transcript_path: PathBuf::from(row.get::<_, String>(3)?),
        title: row.get(4)?,
        subtitle: row.get(5)?,
        first_user_message: row.get(6)?,
        last_user_message: row.get(7)?,
        last_assistant_message: row.get(8)?,
        summary: row.get(9)?,
        kind: row.get(10)?,
        archived: row.get::<_, i64>(11)? != 0,
        has_subagent: row.get::<_, i64>(12)? != 0,
    })
}
