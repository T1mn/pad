use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeminiThreadRef {
    pub session_id: String,
    pub cwd: PathBuf,
    pub updated_at: i64,
    pub transcript_path: PathBuf,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub first_user_message: Option<String>,
    pub last_user_message: Option<String>,
    pub last_assistant_message: Option<String>,
    pub summary: Option<String>,
    pub kind: String,
    pub archived: bool,
    pub has_subagent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GeminiThreadKey {
    pub session_id: String,
    pub cwd: String,
}

#[derive(Clone, Debug)]
pub(crate) struct GeminiSnapshot {
    pub session_id: String,
    pub project_root: PathBuf,
    pub project_alias: String,
    pub transcript_path: PathBuf,
    pub kind: String,
    pub start_time: i64,
    pub last_updated: i64,
    pub summary: Option<String>,
    pub first_user_message: Option<String>,
    pub last_user_message: Option<String>,
    pub last_assistant_message: Option<String>,
    pub payload_hash: String,
}

#[derive(Clone, Debug)]
pub(crate) struct GeminiThreadRecord {
    pub session_id: String,
    pub cwd: PathBuf,
    pub project_alias: String,
    pub transcript_path: PathBuf,
    pub kind: String,
    pub start_time: i64,
    pub updated_at: i64,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub summary: Option<String>,
    pub first_user_message: Option<String>,
    pub last_user_message: Option<String>,
    pub last_assistant_message: Option<String>,
    pub has_subagent: bool,
    pub payload_hash: String,
    pub snapshot_count: i64,
}

impl GeminiThreadKey {
    pub(crate) fn new(session_id: String, cwd: String) -> Self {
        Self { session_id, cwd }
    }
}
