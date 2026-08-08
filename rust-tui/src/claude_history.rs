mod api;
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
mod tests;
