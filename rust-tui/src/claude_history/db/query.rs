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
