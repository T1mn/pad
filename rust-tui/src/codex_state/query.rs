mod db;
mod source;

use super::cache::{load_cached_threads, store_cached_threads};
use super::model::{CodexThreadRef, ThreadArchiveFilter};
use super::pathing::{normalize_path, select_latest_thread_for_cwd};
use source::parse_subagent_parent_thread_id;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) use db::{read_thread_for_id, read_threads_from_db};

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
    Ok(crate::paths::canonical_codex_home_dir().join("state_5.sqlite"))
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
