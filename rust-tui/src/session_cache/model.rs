use crate::hook::HookEvent;
use crate::model::{AgentPanel, AgentType, PreviewTurn, SessionCacheState};
use serde::{Deserialize, Serialize};

pub(super) const CACHE_VERSION: u32 = 1;
pub(super) const RETENTION_SECS: i64 = 30 * 24 * 60 * 60;
pub const SESSION_HISTORY_TURN_LIMIT: usize = 50;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionCacheSnapshot {
    pub agent_session_id: String,
    pub transcript_path: Option<String>,
    pub recent_turns: Vec<PreviewTurn>,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub state: SessionCacheState,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct SessionCacheIndex {
    pub version: u32,
    pub sessions: Vec<CachedSessionRecord>,
    pub pane_bindings: Vec<CachedPaneBinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CachedSessionRecord {
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
pub(super) struct CachedPaneBinding {
    pub agent_session_id: String,
    pub pane_id: String,
    pub session_name: String,
    pub window_index: String,
    pub pane_index: String,
    pub path: String,
    pub agent_type: String,
    pub updated_at: i64,
}

#[derive(Default)]
pub(super) struct HookBindingContext {
    pub session_name: Option<String>,
    pub window_index: Option<String>,
    pub pane_index: Option<String>,
    pub path: Option<String>,
}

impl HookBindingContext {
    pub(super) fn from_event(event: &HookEvent) -> Self {
        Self {
            session_name: event.tmux.session_name.clone(),
            window_index: event.tmux.window_index.clone(),
            pane_index: event.tmux.pane_index.clone(),
            path: event
                .tmux
                .pane_current_path
                .clone()
                .or_else(|| event.cwd.clone()),
        }
    }
}

pub(super) fn supports_cached_session(panel: &AgentPanel) -> bool {
    matches!(
        panel.agent_type,
        AgentType::Claude | AgentType::Codex | AgentType::Gemini
    )
}

pub(super) fn snapshot_from_record(
    record: &CachedSessionRecord,
    state: SessionCacheState,
) -> SessionCacheSnapshot {
    SessionCacheSnapshot {
        agent_session_id: record.agent_session_id.clone(),
        transcript_path: record.transcript_path.clone(),
        recent_turns: record.recent_turns.clone(),
        last_user_prompt: record.last_user_prompt.clone(),
        last_assistant_message: record.last_assistant_message.clone(),
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::supports_cached_session;
    use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

    fn panel(agent_type: AgentType) -> AgentPanel {
        AgentPanel {
            session: "s".into(),
            window: "w".into(),
            window_index: "1".into(),
            pane: "0".into(),
            pane_id: "%1".into(),
            agent_type,
            working_dir: "/tmp".into(),
            is_active: false,
            state: AgentState::Idle,
            state_source: AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        }
    }

    #[test]
    fn gemini_is_supported_by_session_cache() {
        assert!(supports_cached_session(&panel(AgentType::Gemini)));
    }
}
