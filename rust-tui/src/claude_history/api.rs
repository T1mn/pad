use super::db::{
    mutate_thread_archive_state_at, query_thread_for_id_at, query_threads_at,
    upsert_hook_session_at,
};
use super::model::{ClaudeThreadRef, ThreadArchiveFilter, CLAUDE_INDEX_DB_FILE};
use super::scan::sync_index_at;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub fn all_threads() -> io::Result<Vec<ClaudeThreadRef>> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    load_threads_at(&root, &db_path, ThreadArchiveFilter::ActiveOnly)
}

pub fn all_archived_threads() -> io::Result<Vec<ClaudeThreadRef>> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    load_threads_at(&root, &db_path, ThreadArchiveFilter::ArchivedOnly)
}

pub fn thread_for_id(session_id: &str) -> io::Result<Option<ClaudeThreadRef>> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    thread_for_id_at(&root, &db_path, session_id)
}

pub fn upsert_hook_session(
    session_id: &str,
    transcript_path: &Path,
    cwd: &Path,
    title: Option<&str>,
    updated_at: i64,
) -> io::Result<()> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    upsert_hook_session_at(
        &root,
        &db_path,
        session_id,
        transcript_path,
        cwd,
        title,
        updated_at,
    )
}

pub fn archive_thread(session_id: &str) -> io::Result<()> {
    mutate_thread_archive_state(session_id, true)
}

pub fn unarchive_thread(session_id: &str) -> io::Result<()> {
    mutate_thread_archive_state(session_id, false)
}

fn default_projects_root() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(".claude").join("projects"))
}

fn default_index_db_path() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(".pad").join(CLAUDE_INDEX_DB_FILE))
}

pub(crate) fn load_threads_at(
    root: &Path,
    db_path: &Path,
    filter: ThreadArchiveFilter,
) -> io::Result<Vec<ClaudeThreadRef>> {
    sync_index_at(root, db_path)?;
    query_threads_at(root, db_path, filter)
}

pub(crate) fn thread_for_id_at(
    root: &Path,
    db_path: &Path,
    session_id: &str,
) -> io::Result<Option<ClaudeThreadRef>> {
    let started_at = Instant::now();
    if let Some(thread) = query_thread_for_id_at(root, db_path, session_id)? {
        if started_at.elapsed().as_millis() >= 8 {
            crate::log_debug!(
                "claude_history.lookup: session_id={} hit=index elapsed_ms={}",
                session_id,
                started_at.elapsed().as_millis()
            );
        }
        return Ok(Some(thread));
    }
    sync_index_at(root, db_path)?;
    let result = query_thread_for_id_at(root, db_path, session_id)?;
    if started_at.elapsed().as_millis() >= 20 {
        crate::log_debug!(
            "claude_history.lookup: session_id={} hit_after_sync={} elapsed_ms={}",
            session_id,
            result.is_some(),
            started_at.elapsed().as_millis()
        );
    }
    Ok(result)
}

fn mutate_thread_archive_state(session_id: &str, archive: bool) -> io::Result<()> {
    let root = default_projects_root()?;
    let db_path = default_index_db_path()?;
    mutate_thread_archive_state_at(&root, &db_path, session_id, archive)
}
