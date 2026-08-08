mod matching {
    use crate::model::AgentPanel;
    use crate::session_cache::model::CachedPaneBinding;

    const RECENT_BINDING_WINDOW_SECS: i64 = 2 * 60 * 60;

    pub(super) fn exact_binding_matches(
        binding: &CachedPaneBinding,
        panel: &AgentPanel,
        now: i64,
    ) -> bool {
        if binding.pane_id != panel.pane_id {
            return false;
        }

        if pane_pid_matches(binding, panel) {
            return true;
        }

        binding_is_recent(binding, now) && binding_matches_slot(binding, panel)
    }

    pub(super) fn fallback_binding_matches(
        binding: &CachedPaneBinding,
        panel: &AgentPanel,
        now: i64,
    ) -> bool {
        binding_is_recent(binding, now) && binding_matches_slot(binding, panel)
    }

    fn binding_matches_slot(binding: &CachedPaneBinding, panel: &AgentPanel) -> bool {
        binding.path == panel.working_dir
            && binding.session_name == panel.session
            && binding.window_index == panel.window_index
            && binding.pane_index == panel.pane
    }

    fn pane_pid_matches(binding: &CachedPaneBinding, panel: &AgentPanel) -> bool {
        match (binding.pane_pid.as_deref(), panel.pid.as_deref()) {
            (Some(binding_pid), Some(panel_pid)) => {
                !binding_pid.is_empty() && binding_pid == panel_pid
            }
            _ => false,
        }
    }

    fn binding_is_recent(binding: &CachedPaneBinding, now: i64) -> bool {
        binding.updated_at >= now.saturating_sub(RECENT_BINDING_WINDOW_SECS)
    }
}
mod snapshot {
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
}
mod unique {
    use crate::model::AgentPanel;

    pub(super) fn find_unique_session_id<'a>(
        panel: &AgentPanel,
        session_ids: impl Iterator<Item = &'a str>,
    ) -> Option<&'a str> {
        let mut unique = None;

        for session_id in session_ids {
            if is_subagent_session(panel, session_id) {
                continue;
            }
            match unique {
                None => unique = Some(session_id),
                Some(existing) if existing == session_id => {}
                Some(_) => return None,
            }
        }

        unique
    }

    fn is_subagent_session(panel: &AgentPanel, session_id: &str) -> bool {
        matches!(panel.agent_type, crate::model::AgentType::Codex)
            && crate::codex_state::subagent_parent_thread_id(session_id)
                .ok()
                .flatten()
                .is_some()
    }
}

use super::super::model::{SessionCacheIndex, SessionCacheSnapshot};
use super::super::util::now_ts;
use crate::model::{AgentPanel, SessionCacheState};
use std::collections::HashMap;

pub(in crate::session_cache) fn find_snapshot_for_panel(
    index: &SessionCacheIndex,
    panel: &AgentPanel,
) -> Option<SessionCacheSnapshot> {
    let agent_type = panel.agent_type.as_str();
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
