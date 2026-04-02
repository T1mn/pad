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
