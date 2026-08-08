mod bindings;
mod model;
mod persist;
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

use crate::model::AgentType;
use std::collections::HashMap;

pub fn load_snapshots_by_agent_type(
    agent_type: &AgentType,
) -> HashMap<String, SessionCacheSnapshot> {
    let index = storage::load_index();
    bindings::load_snapshots_for_agent_type(&index, agent_type.as_str())
}
