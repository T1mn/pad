use crate::model::SessionCacheState;
use crate::session_cache::model::{
    snapshot_from_record, CachedSessionRecord, SessionCacheIndex, SessionCacheSnapshot,
};
use std::collections::HashMap;

pub(super) fn lookup_snapshot(
    index: &SessionCacheIndex,
    session_id: &str,
    state: SessionCacheState,
) -> Option<SessionCacheSnapshot> {
    index
        .sessions
        .iter()
        .find(|record| record.agent_session_id == session_id)
        .map(|record| snapshot_from_record(record, state))
}

pub(super) fn load_snapshots_for_agent_type(
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
