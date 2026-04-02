use super::model::{ClaudeThreadRef, IndexedClaudeThread, ThreadArchiveFilter, ACTIVE_THREAD_MAX_AGE_SECS};
use super::util::{normalize_path, now_ts, to_io_error};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

pub(crate) fn next_scan_seq(tx: &rusqlite::Transaction<'_>, root_key: &str) -> io::Result<i64> {
    let current = tx
        .query_row(
            "SELECT scan_seq FROM claude_scan_state WHERE root = ?1",
            [root_key],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(to_io_error)?
        .unwrap_or(0);
    Ok(current.saturating_add(1))
}

pub(crate) fn upsert_thread_row(
    tx: &rusqlite::Transaction<'_>,
    root_key: &str,
    thread: &IndexedClaudeThread,
    file_mtime: i64,
    scan_seq: i64,
) -> io::Result<()> {
    tx.execute(
        "INSERT INTO claude_threads (
            root, transcript_path, session_id, cwd, title,
            updated_at, last_assistant_at, file_mtime, last_seen_seq,
            last_seen_at, is_sidechain
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)
         ON CONFLICT(root, transcript_path) DO UPDATE SET
            session_id = excluded.session_id,
            cwd = excluded.cwd,
            title = excluded.title,
            updated_at = excluded.updated_at,
            last_assistant_at = excluded.last_assistant_at,
            file_mtime = excluded.file_mtime,
            last_seen_seq = excluded.last_seen_seq,
            last_seen_at = excluded.last_seen_at,
               is_sidechain = excluded.is_sidechain",
        params![
            root_key,
            thread.transcript_path.to_string_lossy().to_string(),
            thread.session_id,
            normalize_path(&thread.cwd).to_string_lossy().to_string(),
            thread.title,
            thread.updated_at,
            thread.last_assistant_at,
            file_mtime,
            scan_seq,
            now_ts(),
        ],
    )
    .map_err(to_io_error)?;

    Ok(())
}

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
        ThreadArchiveFilter::ArchivedOnly => statement.query_map(params![root_key], map_thread_row),
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

pub(crate) fn mutate_thread_archive_state_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
    archive: bool,
) -> io::Result<()> {
    let root_key = normalize_path(root).to_string_lossy().to_string();
    let connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let updated_at = now_ts();
    let archived_value = if archive { 1_i64 } else { 0_i64 };

    let changed = connection
        .execute(
            "UPDATE claude_threads
             SET archived = ?3,
                 archived_at = CASE WHEN ?3 = 1 THEN ?4 ELSE NULL END,
                 updated_at = MAX(updated_at, ?4)
             WHERE root = ?1
               AND session_id = ?2
               AND archived <> ?3",
            params![root_key, session_id, archived_value, updated_at],
        )
        .map_err(to_io_error)?;
    if changed == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "thread {} was not found or is already in the requested state",
                session_id
            ),
        ));
    }

    Ok(())
}

pub(crate) fn upsert_hook_session_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
    transcript_path: &Path,
    cwd: &Path,
    title: Option<&str>,
    updated_at: i64,
) -> io::Result<()> {
    let started_at = std::time::Instant::now();
    let root_key = normalize_path(root).to_string_lossy().to_string();
    let transcript_key = transcript_path.to_string_lossy().to_string();
    let cwd_key = normalize_path(cwd).to_string_lossy().to_string();

    let mut connection = open_index_db(db_path)?;
    ensure_schema(&connection)?;
    let tx = connection.transaction().map_err(to_io_error)?;

    let existing = tx
        .query_row(
            "SELECT transcript_path
             FROM claude_threads
             WHERE root = ?1 AND session_id = ?2
             ORDER BY updated_at DESC, transcript_path DESC
             LIMIT 1",
            params![root_key, session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_io_error)?;

    if let Some(existing_transcript) = existing {
        tx.execute(
            "UPDATE claude_threads
             SET transcript_path = ?3,
                 cwd = ?4,
                 title = COALESCE(?5, title),
                 updated_at = MAX(updated_at, ?6),
                 last_assistant_at = MAX(last_assistant_at, ?6),
                 file_mtime = MAX(file_mtime, ?6),
                 last_seen_at = ?6
             WHERE root = ?1 AND transcript_path = ?2",
            params![
                root_key,
                existing_transcript,
                transcript_key,
                cwd_key,
                title,
                updated_at,
            ],
        )
        .map_err(to_io_error)?;
    } else {
        tx.execute(
            "INSERT INTO claude_threads (
                root, transcript_path, session_id, cwd, title,
                updated_at, last_assistant_at, file_mtime, last_seen_seq,
                last_seen_at, is_sidechain
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?6, 0, ?6, 0)",
            params![
                root_key,
                transcript_key,
                session_id,
                cwd_key,
                title,
                updated_at
            ],
        )
        .map_err(to_io_error)?;
    }

    tx.commit().map_err(to_io_error)?;
    if started_at.elapsed().as_millis() >= 8 {
        crate::log_debug!(
            "claude_history.hook_upsert: session_id={} elapsed_ms={} transcript={}",
            session_id,
            started_at.elapsed().as_millis(),
            transcript_path.display()
        );
    }
    Ok(())
}
