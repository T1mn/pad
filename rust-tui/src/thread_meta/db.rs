use rusqlite::{Connection, OpenFlags};
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn ensure_schema_at(db_path: &Path) -> io::Result<()> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let connection = open_db(db_path)?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS thread_meta (
                agent_type TEXT NOT NULL,
                thread_id TEXT NOT NULL,
                title_override TEXT,
                generated_title TEXT,
                generated_turn_count INTEGER,
                generated_updated_at INTEGER,
                deleted INTEGER NOT NULL DEFAULT 0,
                deleted_at INTEGER,
                note TEXT,
                pinned INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY(agent_type, thread_id)
            );
            CREATE TABLE IF NOT EXISTS thread_tags (
                agent_type TEXT NOT NULL,
                thread_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY(agent_type, thread_id, tag)
            );
            CREATE INDEX IF NOT EXISTS idx_thread_meta_updated_at
                ON thread_meta(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_thread_tags_thread
                ON thread_tags(agent_type, thread_id);",
        )
        .map_err(to_io_error)?;
    ensure_column(&connection, "generated_title", "TEXT")?;
    ensure_column(&connection, "generated_turn_count", "INTEGER")?;
    ensure_column(&connection, "generated_updated_at", "INTEGER")?;
    ensure_column(&connection, "deleted", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(&connection, "deleted_at", "INTEGER")?;
    Ok(())
}

pub(super) fn open_db(db_path: &Path) -> io::Result<Connection> {
    Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)
}

pub(super) fn db_path() -> PathBuf {
    crate::paths::pad_db_path()
}

pub(super) fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}

fn ensure_column(connection: &Connection, column: &str, definition: &str) -> io::Result<()> {
    let mut statement = connection
        .prepare("PRAGMA table_info(thread_meta)")
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
            &format!("ALTER TABLE thread_meta ADD COLUMN {column} {definition}"),
            [],
        )
        .map_err(to_io_error)?;
    Ok(())
}
