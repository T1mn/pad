use super::super::util::to_io_error;
use rusqlite::{Connection, OpenFlags};
use std::fs;
use std::io;
use std::path::Path;

pub(crate) fn ensure_schema(connection: &Connection) -> io::Result<()> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS claude_threads (
                root TEXT NOT NULL,
                transcript_path TEXT NOT NULL,
                session_id TEXT NOT NULL,
                cwd TEXT NOT NULL,
                title TEXT,
                updated_at INTEGER NOT NULL,
                last_assistant_at INTEGER NOT NULL,
                file_mtime INTEGER NOT NULL,
                last_seen_seq INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL,
                is_sidechain INTEGER NOT NULL DEFAULT 0,
                archived INTEGER NOT NULL DEFAULT 0,
                archived_at INTEGER,
                PRIMARY KEY(root, transcript_path)
            );
            CREATE INDEX IF NOT EXISTS idx_claude_threads_root_session
                ON claude_threads(root, session_id, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_claude_threads_root_cwd
                ON claude_threads(root, cwd, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_claude_threads_root_activity
                ON claude_threads(root, last_assistant_at DESC, updated_at DESC);
            CREATE TABLE IF NOT EXISTS claude_scan_state (
                root TEXT PRIMARY KEY,
                scan_seq INTEGER NOT NULL,
                last_indexed_at INTEGER NOT NULL
            );",
        )
        .map_err(to_io_error)?;
    ensure_optional_column(
        connection,
        "claude_threads",
        "archived",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_optional_column(connection, "claude_threads", "archived_at", "INTEGER")?;
    Ok(())
}

fn ensure_optional_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> io::Result<()> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(to_io_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(to_io_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_io_error)?;
    if columns.iter().any(|existing| existing == column) {
        return Ok(());
    }
    connection
        .execute(
            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
            [],
        )
        .map_err(to_io_error)?;
    Ok(())
}

pub(crate) fn open_index_db(db_path: &Path) -> io::Result<Connection> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;

    connection
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
        .map_err(to_io_error)?;
    Ok(connection)
}
