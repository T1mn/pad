use super::CachedSessionRecord;
use crate::model::{SessionCacheState, SharedPreviewTurns};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionCacheSnapshot {
    pub agent_session_id: String,
    pub transcript_path: Option<String>,
    pub recent_turns: SharedPreviewTurns,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub state: SessionCacheState,
}

pub(in crate::session_cache) fn snapshot_from_record(
    record: &CachedSessionRecord,
    state: SessionCacheState,
) -> SessionCacheSnapshot {
    SessionCacheSnapshot {
        agent_session_id: record.agent_session_id.clone(),
        transcript_path: record.transcript_path.clone(),
        recent_turns: record.recent_turns.clone().into(),
        last_user_prompt: record.last_user_prompt.clone(),
        last_assistant_message: record.last_assistant_message.clone(),
        state,
    }
}
