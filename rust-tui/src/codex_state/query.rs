use super::cache::{load_cached_threads, store_cached_threads};
use super::model::{CodexThreadRef, ThreadArchiveFilter, ACTIVE_THREAD_MAX_AGE_SECS};
use super::pathing::{normalize_path, select_latest_thread_for_cwd};
use super::util::{to_io_error, unix_now_ts};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::Value;
use std::io;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub fn latest_thread_for_cwd(cwd: &Path) -> io::Result<Option<CodexThreadRef>> {
    let threads = load_threads(default_db_path()?, ThreadArchiveFilter::ActiveOnly)?;
    Ok(select_latest_thread_for_cwd(cwd, &threads).cloned())
}

pub fn all_threads() -> io::Result<Vec<CodexThreadRef>> {
    load_threads(default_db_path()?, ThreadArchiveFilter::ActiveOnly)
}

pub fn all_archived_threads() -> io::Result<Vec<CodexThreadRef>> {
    load_threads(default_db_path()?, ThreadArchiveFilter::ArchivedOnly)
}

pub fn threads_for_cwd(cwd: &Path) -> io::Result<Vec<CodexThreadRef>> {
    let threads = load_threads(default_db_path()?, ThreadArchiveFilter::ActiveOnly)?;
    let normalized = normalize_path(cwd);
    Ok(threads
        .into_iter()
        .filter(|thread| normalize_path(&thread.cwd) == normalized)
        .collect())
}

pub fn archived_threads_for_cwd(cwd: &Path) -> io::Result<Vec<CodexThreadRef>> {
    let threads = load_threads(default_db_path()?, ThreadArchiveFilter::ArchivedOnly)?;
    let normalized = normalize_path(cwd);
    Ok(threads
        .into_iter()
        .filter(|thread| normalize_path(&thread.cwd) == normalized)
        .collect())
}

pub fn thread_for_id(thread_id: &str) -> io::Result<Option<CodexThreadRef>> {
    read_thread_for_id(&default_db_path()?, thread_id)
}

pub fn subagent_parent_thread_id(thread_id: &str) -> io::Result<Option<String>> {
    let Some(thread) = thread_for_id(thread_id)? else {
        return Ok(None);
    };
    Ok(parse_subagent_parent_thread_id(thread.source.as_deref()))
}

pub(crate) fn default_db_path() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(".codex").join("state_5.sqlite"))
}

pub(crate) fn load_threads(
    db_path: PathBuf,
    filter: ThreadArchiveFilter,
) -> io::Result<Vec<CodexThreadRef>> {
    if let Some(cached) = load_cached_threads(&db_path, filter) {
        return Ok(cached);
    }

    let threads = read_threads_from_db(&db_path, filter)?;
    store_cached_threads(db_path, filter, &threads);
    Ok(threads)
}

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
    let sql = match filter {
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
        ThreadArchiveFilter::All => {
            "SELECT id, cwd, updated_at, rollout_path, title, first_user_message, source, archived
             FROM threads
             WHERE rollout_path IS NOT NULL
               AND TRIM(rollout_path) <> ''
             ORDER BY updated_at DESC, id DESC"
        }
    };
    let mut statement = connection.prepare(sql).map_err(to_io_error)?;
    let mapper = |row: &rusqlite::Row<'_>| {
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
    };
    let rows = match filter {
        ThreadArchiveFilter::ActiveOnly => statement.query_map([min_updated_at], mapper),
        ThreadArchiveFilter::ArchivedOnly | ThreadArchiveFilter::All => {
            statement.query_map([], mapper)
        }
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
            |row| {
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
            },
        )
        .optional()
        .map_err(to_io_error)
}

fn parse_subagent_parent_thread_id(source: Option<&str>) -> Option<String> {
    let source = source?.trim();
    if source.is_empty() || !source.starts_with('{') {
        return None;
    }

    let value = serde_json::from_str::<Value>(source).ok()?;
    value
        .get("subagent")
        .and_then(|subagent| subagent.get("thread_spawn"))
        .and_then(|spawn| spawn.get("parent_thread_id"))
        .and_then(|parent| parent.as_str())
        .map(|parent| parent.to_string())
}
