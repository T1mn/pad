use crate::model::PreviewTurn;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(in crate::session_cache) struct SessionCacheIndex {
    pub version: u32,
    pub sessions: Vec<CachedSessionRecord>,
    pub pane_bindings: Vec<CachedPaneBinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::session_cache) struct CachedSessionRecord {
    pub agent_session_id: String,
    pub agent_type: String,
    pub transcript_path: Option<String>,
    pub recent_turns: Vec<PreviewTurn>,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub last_seen_at: i64,
    pub updated_at: i64,
    pub last_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::session_cache) struct CachedPaneBinding {
    pub agent_session_id: String,
    pub pane_id: String,
    pub pane_pid: Option<String>,
    pub session_name: String,
    pub window_index: String,
    pub pane_index: String,
    pub path: String,
    pub agent_type: String,
    pub updated_at: i64,
}
