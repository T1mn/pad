use crate::model::{
    AgentState, AgentType, PreviewSessionOrigin, PreviewSource, SessionCacheState,
    SharedPreviewTurns,
};

#[derive(Clone, Debug)]
pub struct PreviewRequest {
    pub target_key: String,
    pub live_pane_id: Option<String>,
    pub agent_type: AgentType,
    pub working_dir: String,
    pub state: AgentState,
    pub transcript_path: Option<String>,
    pub cached_preview_turns: SharedPreviewTurns,
    pub session_cache_state: Option<SessionCacheState>,
    pub agent_session_id: Option<String>,
    pub session_origin: Option<PreviewSessionOrigin>,
    pub persist_resolved_session: bool,
    pub known_updated_at: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct PreviewUpdate {
    pub target_key: String,
    pub live_pane_id: Option<String>,
    pub content: String,
    pub source: PreviewSource,
    pub session_origin: Option<PreviewSessionOrigin>,
    pub session_id: Option<String>,
    pub turns: SharedPreviewTurns,
    pub transcript_path: Option<String>,
    pub session_cache_state: Option<SessionCacheState>,
    pub updated_at: Option<i64>,
}
