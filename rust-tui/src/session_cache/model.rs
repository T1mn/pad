mod context {
    use crate::hook::HookEvent;

    #[derive(Default)]
    pub(in crate::session_cache) struct HookBindingContext {
        pub session_name: Option<String>,
        pub window_index: Option<String>,
        pub pane_index: Option<String>,
        pub path: Option<String>,
    }

    impl HookBindingContext {
        pub(in crate::session_cache) fn from_event(event: &HookEvent) -> Self {
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
}
mod records {
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
mod support;

pub(super) const CACHE_VERSION: u32 = 1;
pub(super) const RETENTION_SECS: i64 = 30 * 24 * 60 * 60;
pub const SESSION_HISTORY_TURN_LIMIT: usize = 50;

pub(super) use context::HookBindingContext;
pub(super) use records::{CachedPaneBinding, CachedSessionRecord, SessionCacheIndex};
pub(super) use snapshot::snapshot_from_record;
pub use snapshot::SessionCacheSnapshot;
pub(super) use support::supports_cached_session;
