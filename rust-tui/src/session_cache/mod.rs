mod bindings;
mod model;
mod persist;
mod preload;
mod storage;
mod tests;
mod util;

pub use model::{SessionCacheSnapshot, SESSION_HISTORY_TURN_LIMIT};
pub use persist::{persist_hook_event, persist_resolved_session};
pub use preload::preload_panels;

use crate::model::AgentType;
use std::collections::HashMap;

pub fn load_snapshots_by_agent_type(
    agent_type: &AgentType,
) -> HashMap<String, SessionCacheSnapshot> {
    let index = storage::load_index();
    bindings::load_snapshots_for_agent_type(&index, &agent_type.to_string())
}
