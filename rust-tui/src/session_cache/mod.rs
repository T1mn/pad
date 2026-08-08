mod bindings;
mod model;
mod persist;
mod preload;
mod storage;
mod tests;
mod turns;
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
pub use preload::preload_panels;

use crate::model::AgentType;
use std::collections::HashMap;

pub fn load_snapshots_by_agent_type(
    agent_type: &AgentType,
) -> HashMap<String, SessionCacheSnapshot> {
    let index = storage::load_index();
    bindings::load_snapshots_for_agent_type(&index, agent_type.as_str())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedSessionSummary {
    pub agent_session_id: String,
    pub agent_type: String,
    pub transcript_path: Option<String>,
    pub working_dir: Option<String>,
    pub pane_id: Option<String>,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub updated_at: i64,
    pub last_seen_at: i64,
}

pub fn list_cached_sessions() -> Vec<CachedSessionSummary> {
    let index = storage::load_index();
    let bindings = latest_bindings_by_session(&index);
    index
        .sessions
        .iter()
        .map(|record| {
            cached_session_summary(
                record,
                bindings.get(record.agent_session_id.as_str()).copied(),
            )
        })
        .collect()
}

pub fn find_cached_session(session_id: &str) -> Option<CachedSessionSummary> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return None;
    }

    let index = storage::load_index();
    index
        .sessions
        .iter()
        .find(|record| record.agent_session_id == session_id)
        .map(|record| {
            cached_session_summary(record, latest_binding_for_session(&index, session_id))
        })
}

fn cached_session_summary(
    record: &model::CachedSessionRecord,
    binding: Option<&model::CachedPaneBinding>,
) -> CachedSessionSummary {
    CachedSessionSummary {
        agent_session_id: record.agent_session_id.clone(),
        agent_type: record.agent_type.clone(),
        transcript_path: record.transcript_path.clone(),
        working_dir: binding.map(|binding| binding.path.clone()),
        pane_id: binding.map(|binding| binding.pane_id.clone()),
        last_user_prompt: record.last_user_prompt.clone(),
        last_assistant_message: record.last_assistant_message.clone(),
        updated_at: record.updated_at,
        last_seen_at: record.last_seen_at,
    }
}

fn latest_bindings_by_session(
    index: &model::SessionCacheIndex,
) -> HashMap<&str, &model::CachedPaneBinding> {
    let mut latest = HashMap::with_capacity(index.pane_bindings.len());
    for binding in &index.pane_bindings {
        let entry = latest
            .entry(binding.agent_session_id.as_str())
            .or_insert(binding);
        if binding.updated_at > entry.updated_at {
            *entry = binding;
        }
    }
    latest
}

fn latest_binding_for_session<'a>(
    index: &'a model::SessionCacheIndex,
    session_id: &str,
) -> Option<&'a model::CachedPaneBinding> {
    index
        .pane_bindings
        .iter()
        .filter(|binding| binding.agent_session_id == session_id)
        .max_by_key(|binding| binding.updated_at)
}
