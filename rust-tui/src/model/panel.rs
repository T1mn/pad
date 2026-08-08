use super::{AgentState, AgentType, SessionCacheState, SharedPreviewTurns};

#[derive(Clone, Debug)]
pub struct AgentPanel {
    pub session: String,
    pub window: String,
    pub window_index: String,
    pub pane: String,
    pub pane_id: String,
    pub agent_type: AgentType,
    pub working_dir: String,
    pub is_active: bool,
    pub state: AgentState,
    pub transcript_path: Option<String>,
    pub cached_preview_turns: SharedPreviewTurns,
    pub session_cache_state: Option<SessionCacheState>,
    pub agent_session_id: Option<String>,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub has_unread_stop: bool,
}
