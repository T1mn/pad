use super::model::{GeminiThreadRecord, GeminiThreadRef};
use super::util::{ensure_parent_dir, normalize_path};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use std::io;
use std::path::{Path, PathBuf};

const DB_TABLE: &str = "gemini_threads";

pub(crate) fn replace_records(db_path: &Path, records: &[GeminiThreadRecord]) -> io::Result<()> {
    ensure_parent_dir(db_path)?;
    let mut connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let tx = connection.transaction().map_err(to_io_error)?;

    for record in records {
        upsert_record(&tx, record)?;
    }

    tx.commit().map_err(to_io_error)
}

pub(crate) fn query_threads(
    db_path: &Path,
    archived: Option<bool>,
) -> io::Result<Vec<GeminiThreadRef>> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let sql = match archived {
        Some(true) => format!(
            "SELECT session_id, cwd, updated_at, transcript_path, title, subtitle,
                    first_user_message, last_user_message, last_assistant_message,
                    summary, kind, archived, has_subagent
             FROM {DB_TABLE}
             WHERE archived = 1
             ORDER BY updated_at DESC, session_id DESC, cwd DESC"
        ),
        Some(false) => format!(
            "SELECT session_id, cwd, updated_at, transcript_path, title, subtitle,
                    first_user_message, last_user_message, last_assistant_message,
                    summary, kind, archived, has_subagent
             FROM {DB_TABLE}
             WHERE archived = 0
             ORDER BY updated_at DESC, session_id DESC, cwd DESC"
        ),
        None => format!(
            "SELECT session_id, cwd, updated_at, transcript_path, title, subtitle,
                    first_user_message, last_user_message, last_assistant_message,
                    summary, kind, archived, has_subagent
             FROM {DB_TABLE}
             ORDER BY updated_at DESC, session_id DESC, cwd DESC"
        ),
    };
    let mut statement = connection.prepare(&sql).map_err(to_io_error)?;
    let rows = statement
        .query_map([], map_row)
        .map_err(to_io_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_io_error)?;
    Ok(rows)
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
                "SELECT session_id, cwd, updated_at, transcript_path, title, subtitle,
                        first_user_message, last_user_message, last_assistant_message,
                        summary, kind, archived, has_subagent
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
    let sql = match archived {
        Some(true) => format!(
            "SELECT session_id, cwd, updated_at, transcript_path, title, subtitle,
                    first_user_message, last_user_message, last_assistant_message,
                    summary, kind, archived, has_subagent
             FROM {DB_TABLE}
             WHERE cwd = ?1 AND archived = 1
             ORDER BY updated_at DESC, session_id DESC"
        ),
        Some(false) => format!(
            "SELECT session_id, cwd, updated_at, transcript_path, title, subtitle,
                    first_user_message, last_user_message, last_assistant_message,
                    summary, kind, archived, has_subagent
             FROM {DB_TABLE}
             WHERE cwd = ?1 AND archived = 0
             ORDER BY updated_at DESC, session_id DESC"
        ),
        None => format!(
            "SELECT session_id, cwd, updated_at, transcript_path, title, subtitle,
                    first_user_message, last_user_message, last_assistant_message,
                    summary, kind, archived, has_subagent
             FROM {DB_TABLE}
             WHERE cwd = ?1
             ORDER BY updated_at DESC, session_id DESC"
        ),
    };
    let mut statement = connection.prepare(&sql).map_err(to_io_error)?;
    let rows = statement
        .query_map([cwd_key], map_row)
        .map_err(to_io_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_io_error)?;
    Ok(rows)
}

pub(crate) fn set_threads_archived(
    db_path: &Path,
    session_id: &str,
    archived: bool,
) -> io::Result<()> {
    if !db_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Gemini index not found at {}", db_path.display()),
        ));
    }

    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let archived_value = if archived { 1_i64 } else { 0_i64 };
    let changed = connection
        .execute(
            &format!(
                "UPDATE {DB_TABLE}
                 SET archived = ?2
                 WHERE session_id = ?1"
            ),
            params![session_id, archived_value],
        )
        .map_err(to_io_error)?;

    if changed == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("thread {} was not found", session_id),
        ));
    }

    Ok(())
}

fn upsert_record(tx: &rusqlite::Transaction<'_>, record: &GeminiThreadRecord) -> io::Result<()> {
    tx.execute(
        &format!(
            "INSERT INTO {DB_TABLE} (
                session_id, cwd, project_alias, transcript_path, kind, start_time,
                updated_at, title, subtitle, summary, first_user_message,
                last_user_message, last_assistant_message, archived, has_subagent,
                payload_hash, snapshot_count
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, COALESCE((SELECT archived FROM {DB_TABLE}
                    WHERE session_id = ?1 AND cwd = ?2), 0), ?14,
                ?15, ?16
            )
            ON CONFLICT(session_id, cwd) DO UPDATE SET
                project_alias = excluded.project_alias,
                transcript_path = excluded.transcript_path,
                kind = excluded.kind,
                start_time = excluded.start_time,
                updated_at = excluded.updated_at,
                title = excluded.title,
                subtitle = excluded.subtitle,
                summary = excluded.summary,
                first_user_message = excluded.first_user_message,
                last_user_message = excluded.last_user_message,
                last_assistant_message = excluded.last_assistant_message,
                has_subagent = excluded.has_subagent,
                payload_hash = excluded.payload_hash,
                snapshot_count = excluded.snapshot_count"
        ),
        params![
            record.session_id,
            record.cwd.to_string_lossy().to_string(),
            record.project_alias,
            record.transcript_path.to_string_lossy().to_string(),
            record.kind,
            record.start_time,
            record.updated_at,
            record.title,
            record.subtitle,
            record.summary,
            record.first_user_message,
            record.last_user_message,
            record.last_assistant_message,
            if record.has_subagent { 1_i64 } else { 0_i64 },
            record.payload_hash,
            record.snapshot_count,
        ],
    )
    .map_err(to_io_error)?;
    Ok(())
}

pub(crate) fn ensure_schema(connection: &Connection) -> io::Result<()> {
    connection
        .execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {DB_TABLE} (
                    session_id TEXT NOT NULL,
                    cwd TEXT NOT NULL,
                    project_alias TEXT NOT NULL,
                    transcript_path TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    start_time INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    title TEXT,
                    subtitle TEXT,
                    summary TEXT,
                    first_user_message TEXT,
                    last_user_message TEXT,
                    last_assistant_message TEXT,
                    archived INTEGER NOT NULL DEFAULT 0,
                    has_subagent INTEGER NOT NULL DEFAULT 0,
                    payload_hash TEXT NOT NULL,
                    snapshot_count INTEGER NOT NULL DEFAULT 1,
                    PRIMARY KEY(session_id, cwd)
                );
                CREATE INDEX IF NOT EXISTS idx_gemini_threads_updated_at
                    ON {DB_TABLE}(updated_at DESC);
                CREATE INDEX IF NOT EXISTS idx_gemini_threads_cwd
                    ON {DB_TABLE}(cwd);
                CREATE INDEX IF NOT EXISTS idx_gemini_threads_archived
                    ON {DB_TABLE}(archived, updated_at DESC);
                CREATE INDEX IF NOT EXISTS idx_gemini_threads_session_id
                    ON {DB_TABLE}(session_id);"
        ))
        .map_err(to_io_error)?;

    ensure_column(connection, "project_alias", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(connection, "transcript_path", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(connection, "kind", "TEXT NOT NULL DEFAULT 'main'")?;
    ensure_column(connection, "start_time", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(connection, "updated_at", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(connection, "title", "TEXT")?;
    ensure_column(connection, "subtitle", "TEXT")?;
    ensure_column(connection, "summary", "TEXT")?;
    ensure_column(connection, "first_user_message", "TEXT")?;
    ensure_column(connection, "last_user_message", "TEXT")?;
    ensure_column(connection, "last_assistant_message", "TEXT")?;
    ensure_column(connection, "archived", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(connection, "has_subagent", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(connection, "payload_hash", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(connection, "snapshot_count", "INTEGER NOT NULL DEFAULT 1")?;

    Ok(())
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

fn open_index_db(db_path: &Path) -> io::Result<Connection> {
    ensure_parent_dir(db_path)?;
    Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)
}

fn ensure_column(connection: &Connection, column: &str, definition: &str) -> io::Result<()> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({DB_TABLE})"))
        .map_err(to_io_error)?;
    let existing = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(to_io_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_io_error)?;

    if existing.iter().any(|name| name == column) {
        return Ok(());
    }

    connection
        .execute(
            &format!("ALTER TABLE {DB_TABLE} ADD COLUMN {column} {definition}"),
            [],
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}
