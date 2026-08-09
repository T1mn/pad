mod lookup {
    use super::super::model::{snapshot_from_record, CachedSessionRecord, SessionCacheIndex};
    use crate::model::SessionCacheState;
    use crate::session_cache::SessionCacheSnapshot;
    use std::collections::HashMap;

    pub(in crate::session_cache) fn load_snapshots_for_agent_type(
        index: &SessionCacheIndex,
        agent_type: &str,
    ) -> HashMap<String, SessionCacheSnapshot> {
        index
            .sessions
            .iter()
            .filter(|record| record.agent_type == agent_type)
            .map(|record| {
                (
                    record.agent_session_id.clone(),
                    snapshot_from_record(record, snapshot_state(record)),
                )
            })
            .collect()
    }

    fn snapshot_state(record: &CachedSessionRecord) -> SessionCacheState {
        match record.last_source.as_str() {
            "resolver" => SessionCacheState::Confirmed,
            _ => SessionCacheState::Cached,
        }
    }
}

pub(super) use lookup::load_snapshots_for_agent_type;
