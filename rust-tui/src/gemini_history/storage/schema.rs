use super::super::util::ensure_parent_dir;
use rusqlite::{Connection, OpenFlags};
use std::io;
use std::path::Path;

pub(super) const DB_TABLE: &str = "gemini_threads";

pub(super) fn ensure_schema(connection: &Connection) -> io::Result<()> {
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

pub(super) fn open_index_db(db_path: &Path) -> io::Result<Connection> {
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

pub(super) fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}
