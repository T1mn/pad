mod api {
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
        Ok(crate::paths::claude_projects_dir())
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
}
mod db;
mod model {
    use std::path::PathBuf;

    pub(crate) const ACTIVE_THREAD_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;
    pub(crate) const CLAUDE_INDEX_DB_FILE: &str = "claude_history.sqlite";

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ClaudeThreadRef {
        pub session_id: String,
        pub cwd: PathBuf,
        pub updated_at: i64,
        pub transcript_path: PathBuf,
        pub title: Option<String>,
        pub archived: bool,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct IndexedClaudeThread {
        pub session_id: String,
        pub cwd: PathBuf,
        pub transcript_path: PathBuf,
        pub title: Option<String>,
        pub updated_at: i64,
        pub last_assistant_at: i64,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum ThreadArchiveFilter {
        ActiveOnly,
        ArchivedOnly,
    }
}
mod parse;
mod scan;
mod util {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    pub(crate) fn file_mtime_secs(path: &Path) -> io::Result<i64> {
        fs::metadata(path)?
            .modified()
            .ok()
            .and_then(crate::time::system_time_unix_secs)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "failed to read file mtime"))
    }

    pub(crate) fn now_ts() -> i64 {
        crate::time::unix_now_ts()
    }

    pub(crate) fn normalize_path(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    pub(crate) fn to_io_error(err: rusqlite::Error) -> io::Error {
        io::Error::other(err)
    }
}

pub use api::{
    all_archived_threads, all_threads, archive_thread, thread_for_id, unarchive_thread,
    upsert_hook_session,
};
pub use model::ClaudeThreadRef;

#[cfg(test)]
pub(crate) mod tests;
