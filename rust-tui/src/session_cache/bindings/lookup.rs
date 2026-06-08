mod matching;
mod snapshot;
mod unique;

use super::super::model::{SessionCacheIndex, SessionCacheSnapshot};
use super::super::util::now_ts;
use crate::model::{AgentPanel, SessionCacheState};
use std::collections::HashMap;

pub(in crate::session_cache) fn find_snapshot_for_panel(
    index: &SessionCacheIndex,
    panel: &AgentPanel,
) -> Option<SessionCacheSnapshot> {
    let agent_type = panel.agent_type.to_string();
    let now = now_ts();

    let exact_match = unique::find_unique_session_id(
        panel,
        index.pane_bindings.iter().filter_map(|binding| {
            (binding.agent_type == agent_type
                && matching::exact_binding_matches(binding, panel, now))
            .then_some(binding.agent_session_id.as_str())
        }),
    );

    if let Some(session_id) = exact_match {
        return snapshot::lookup_snapshot(index, session_id, SessionCacheState::Cached);
    }

    let fallback_match = unique::find_unique_session_id(
        panel,
        index.pane_bindings.iter().filter_map(|binding| {
            (binding.agent_type == agent_type
                && matching::fallback_binding_matches(binding, panel, now))
            .then_some(binding.agent_session_id.as_str())
        }),
    );

    if let Some(session_id) = fallback_match {
        return snapshot::lookup_snapshot(index, session_id, SessionCacheState::Cached);
    }

    None
}

pub(in crate::session_cache) fn load_snapshots_for_agent_type(
    index: &SessionCacheIndex,
    agent_type: &str,
) -> HashMap<String, SessionCacheSnapshot> {
    snapshot::load_snapshots_for_agent_type(index, agent_type)
}
