mod query {
    use super::super::model::{ClaudeThreadRef, ThreadArchiveFilter, ACTIVE_THREAD_MAX_AGE_SECS};
    use super::super::util::{normalize_path, now_ts, to_io_error};
    use super::schema::{ensure_schema, open_index_db};
    use rusqlite::{params, OptionalExtension};
    use std::io;
    use std::path::{Path, PathBuf};

    pub(crate) fn query_threads_at(
        root: &Path,
        db_path: &Path,
        filter: ThreadArchiveFilter,
    ) -> io::Result<Vec<ClaudeThreadRef>> {
        let connection = open_index_db(db_path)?;
        ensure_schema(&connection)?;
        let root_key = normalize_path(root).to_string_lossy().to_string();
        let min_assistant_ts = now_ts().saturating_sub(ACTIVE_THREAD_MAX_AGE_SECS);

        let sql = match filter {
            ThreadArchiveFilter::ActiveOnly => {
                "SELECT session_id, cwd, updated_at, transcript_path, title, archived
                 FROM claude_threads
                 WHERE root = ?1
                   AND archived = 0
                   AND last_assistant_at >= ?2
                 ORDER BY updated_at DESC, transcript_path DESC"
            }
            ThreadArchiveFilter::ArchivedOnly => {
                "SELECT session_id, cwd, updated_at, transcript_path, title, archived
                 FROM claude_threads
                 WHERE root = ?1
                   AND archived = 1
                 ORDER BY updated_at DESC, transcript_path DESC"
            }
        };
        let mut statement = connection.prepare(sql).map_err(to_io_error)?;
        let rows = match filter {
            ThreadArchiveFilter::ActiveOnly => {
                statement.query_map(params![root_key, min_assistant_ts], map_thread_row)
            }
            ThreadArchiveFilter::ArchivedOnly => {
                statement.query_map(params![root_key], map_thread_row)
            }
        }
        .map_err(to_io_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(to_io_error)
    }

    pub(crate) fn query_thread_for_id_at(
        root: &Path,
        db_path: &Path,
        session_id: &str,
    ) -> io::Result<Option<ClaudeThreadRef>> {
        let connection = open_index_db(db_path)?;
        ensure_schema(&connection)?;
        let root_key = normalize_path(root).to_string_lossy().to_string();

        connection
            .query_row(
                "SELECT session_id, cwd, updated_at, transcript_path, title, archived
                 FROM claude_threads
                 WHERE root = ?1
                   AND session_id = ?2
                 ORDER BY updated_at DESC, transcript_path DESC
                 LIMIT 1",
                params![root_key, session_id],
                map_thread_row,
            )
            .optional()
            .map_err(to_io_error)
    }

    fn map_thread_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClaudeThreadRef> {
        Ok(ClaudeThreadRef {
            session_id: row.get::<_, String>(0)?,
            cwd: PathBuf::from(row.get::<_, String>(1)?),
            updated_at: row.get::<_, i64>(2)?,
            transcript_path: PathBuf::from(row.get::<_, String>(3)?),
            title: row.get::<_, Option<String>>(4)?,
            archived: row.get::<_, i64>(5).unwrap_or_default() != 0,
        })
    }
}
mod schema {
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
}
mod write;

pub(crate) use query::{query_thread_for_id_at, query_threads_at};
pub(crate) use schema::{ensure_schema, open_index_db};
pub(crate) use write::{
    mutate_thread_archive_state_at, next_scan_seq, upsert_hook_session_at, upsert_thread_row,
};
