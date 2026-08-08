use std::collections::HashMap;
use std::io;

mod db {
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
}
mod model {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ThreadMetaKey {
        pub agent_type: String,
        pub thread_id: String,
    }

    impl ThreadMetaKey {
        pub fn new(agent_type: impl Into<String>, thread_id: impl Into<String>) -> Self {
            Self {
                agent_type: agent_type.into(),
                thread_id: thread_id.into(),
            }
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct ThreadMeta {
        pub title_override: Option<String>,
        pub generated_title: Option<String>,
        pub generated_turn_count: Option<usize>,
        pub generated_updated_at: Option<i64>,
        pub deleted: bool,
        pub deleted_at: Option<i64>,
        pub note: Option<String>,
        pub pinned: bool,
        pub tags: Vec<String>,
        pub updated_at: i64,
    }
}
mod storage;
#[cfg(test)]
mod tests;

pub use model::{ThreadMeta, ThreadMetaKey};

use db::{db_path, ensure_schema_at};
use storage::{
    deleted_thread_count_at, load_deleted_thread_meta_at, load_thread_meta_batch_at,
    replace_thread_tags_at, set_thread_deleted_at, upsert_generated_title_at,
    upsert_thread_meta_at,
};

pub fn ensure_db() -> io::Result<()> {
    ensure_schema_at(&db_path())
}

pub fn load_thread_meta_batch(
    keys: &[ThreadMetaKey],
) -> io::Result<HashMap<ThreadMetaKey, ThreadMeta>> {
    load_thread_meta_batch_at(&db_path(), keys)
}

pub fn load_thread_meta(agent_type: &str, thread_id: &str) -> io::Result<Option<ThreadMeta>> {
    let key = ThreadMetaKey::new(agent_type, thread_id);
    Ok(load_thread_meta_batch(std::slice::from_ref(&key))?.remove(&key))
}

pub fn upsert_thread_meta(
    agent_type: &str,
    thread_id: &str,
    title_override: Option<&str>,
    note: Option<&str>,
    pinned: bool,
) -> io::Result<()> {
    upsert_thread_meta_at(
        &db_path(),
        agent_type,
        thread_id,
        title_override,
        note,
        pinned,
    )
}

pub fn upsert_generated_title(
    agent_type: &str,
    thread_id: &str,
    generated_title: &str,
    generated_turn_count: usize,
) -> io::Result<()> {
    upsert_generated_title_at(
        &db_path(),
        agent_type,
        thread_id,
        generated_title,
        generated_turn_count,
    )
}

pub fn set_thread_deleted(agent_type: &str, thread_id: &str, deleted: bool) -> io::Result<()> {
    set_thread_deleted_at(&db_path(), agent_type, thread_id, deleted)
}

pub fn deleted_thread_count() -> io::Result<usize> {
    deleted_thread_count_at(&db_path())
}

pub fn load_deleted_thread_meta() -> io::Result<Vec<(ThreadMetaKey, ThreadMeta)>> {
    load_deleted_thread_meta_at(&db_path())
}

pub fn replace_thread_tags(agent_type: &str, thread_id: &str, tags: &[String]) -> io::Result<()> {
    replace_thread_tags_at(&db_path(), agent_type, thread_id, tags)
}
