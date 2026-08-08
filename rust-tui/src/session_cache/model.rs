mod records {
    use crate::model::PreviewTurn;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub(in crate::session_cache) struct SessionCacheIndex {
        pub version: u32,
        pub sessions: Vec<CachedSessionRecord>,
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
}
mod snapshot {
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
}
pub(super) const CACHE_VERSION: u32 = 1;
pub(super) const RETENTION_SECS: i64 = 30 * 24 * 60 * 60;
pub const SESSION_HISTORY_TURN_LIMIT: usize = 50;

pub(super) use records::{CachedSessionRecord, SessionCacheIndex};
pub(super) use snapshot::snapshot_from_record;
pub use snapshot::SessionCacheSnapshot;
