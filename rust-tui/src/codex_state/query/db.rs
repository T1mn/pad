use super::super::model::{CodexThreadRef, ThreadArchiveFilter, ACTIVE_THREAD_MAX_AGE_SECS};
use super::super::util::{to_io_error, unix_now_ts};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn read_threads_from_db(
    db_path: &Path,
    filter: ThreadArchiveFilter,
) -> io::Result<Vec<CodexThreadRef>> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;

    let min_updated_at = unix_now_ts().saturating_sub(ACTIVE_THREAD_MAX_AGE_SECS);
    let sql = threads_query_sql(filter);
    let mut statement = connection.prepare(sql).map_err(to_io_error)?;
    let rows = match filter {
        ThreadArchiveFilter::ActiveOnly => statement.query_map([min_updated_at], map_thread_row),
        ThreadArchiveFilter::ArchivedOnly => statement.query_map([], map_thread_row),
    }
    .map_err(to_io_error)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(to_io_error)
}

pub(crate) fn read_thread_for_id(
    db_path: &Path,
    thread_id: &str,
) -> io::Result<Option<CodexThreadRef>> {
    if !db_path.exists() {
        return Ok(None);
    }

    let connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;

    connection
        .query_row(
            "SELECT id, cwd, updated_at, rollout_path, title, first_user_message, source, archived
             FROM threads
             WHERE id = ?1
               AND rollout_path IS NOT NULL
               AND TRIM(rollout_path) <> ''",
            [thread_id],
            map_thread_row,
        )
        .optional()
        .map_err(to_io_error)
}

fn threads_query_sql(filter: ThreadArchiveFilter) -> &'static str {
    match filter {
        ThreadArchiveFilter::ActiveOnly => {
            "SELECT id, cwd, updated_at, rollout_path, title, first_user_message, source, archived
             FROM threads
             WHERE rollout_path IS NOT NULL
               AND TRIM(rollout_path) <> ''
               AND archived = 0
               AND updated_at >= ?1
             ORDER BY updated_at DESC, id DESC"
        }
        ThreadArchiveFilter::ArchivedOnly => {
            "SELECT id, cwd, updated_at, rollout_path, title, first_user_message, source, archived
             FROM threads
             WHERE rollout_path IS NOT NULL
               AND TRIM(rollout_path) <> ''
               AND archived = 1
             ORDER BY updated_at DESC, id DESC"
        }
    }
}

fn map_thread_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CodexThreadRef> {
    Ok(CodexThreadRef {
        thread_id: row.get::<_, String>(0)?,
        cwd: PathBuf::from(row.get::<_, String>(1)?),
        updated_at: row.get::<_, i64>(2)?,
        rollout_path: PathBuf::from(row.get::<_, String>(3)?),
        title: row.get::<_, Option<String>>(4)?,
        first_user_message: row.get::<_, Option<String>>(5)?,
        source: row.get::<_, Option<String>>(6)?,
        archived: row.get::<_, i64>(7).unwrap_or_default() != 0,
    })
}
