mod bindings;
mod model {
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
}
mod persist;
mod storage {
    use super::model::{SessionCacheIndex, CACHE_VERSION, RETENTION_SECS};
    use super::util::now_ts;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    pub(super) fn load_index() -> SessionCacheIndex {
        let path = crate::paths::sessions_index_path();
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => {
                return SessionCacheIndex {
                    version: CACHE_VERSION,
                    ..SessionCacheIndex::default()
                };
            }
        };

        serde_json::from_str(&content).unwrap_or_else(|err| {
            log_debug!("session_cache: failed to parse {}: {}", path.display(), err);
            SessionCacheIndex {
                version: CACHE_VERSION,
                ..SessionCacheIndex::default()
            }
        })
    }

    pub(super) fn save_index(index: &SessionCacheIndex) -> io::Result<()> {
        let path = crate::paths::sessions_index_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let tmp_path = temporary_index_path(&path);
        let content = serde_json::to_string_pretty(index)?;
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    pub(super) fn prune_index(index: &mut SessionCacheIndex) -> bool {
        if index.version != CACHE_VERSION {
            index.version = CACHE_VERSION;
        }

        let now = now_ts();
        let min_ts = now.saturating_sub(RETENTION_SECS);

        let before_sessions = index.sessions.len();
        index.sessions.retain(|record| {
            if record.updated_at < min_ts {
                return false;
            }
            if let Some(path) = record.transcript_path.as_deref() {
                return Path::new(path).exists();
            }
            true
        });

        before_sessions != index.sessions.len()
    }

    fn temporary_index_path(path: &Path) -> PathBuf {
        let pid = std::process::id();
        let stamp = now_ts();
        path.with_extension(format!("tmp.{}.{}", pid, stamp))
    }
}
pub(crate) mod tests;
pub(crate) mod turns;
mod util {
    pub(super) fn first_non_empty_str<'a>(
        values: impl IntoIterator<Item = Option<&'a str>>,
    ) -> Option<&'a str> {
        values
            .into_iter()
            .flatten()
            .map(str::trim)
            .find(|text| !text.is_empty())
    }

    pub(super) fn prefer_non_empty_str<'a>(
        values: impl IntoIterator<Item = Option<&'a str>>,
    ) -> Option<String> {
        first_non_empty_str(values).map(ToOwned::to_owned)
    }

    pub(super) fn clean_text(value: Option<&str>) -> Option<String> {
        value
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
    }

    pub(super) fn now_ts() -> i64 {
        crate::time::unix_now_ts()
    }
}

pub use model::{SessionCacheSnapshot, SESSION_HISTORY_TURN_LIMIT};
pub use persist::{persist_hook_event, persist_resolved_session};

use crate::model::AgentType;
use std::collections::HashMap;

pub fn load_snapshots_by_agent_type(
    agent_type: &AgentType,
) -> HashMap<String, SessionCacheSnapshot> {
    let index = storage::load_index();
    bindings::load_snapshots_for_agent_type(&index, agent_type.as_str())
}
