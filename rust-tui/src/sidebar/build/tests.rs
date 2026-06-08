use super::activity::merge_or_insert_thread;
use super::history_codex::build_codex_history_entry;
use super::meta::apply_thread_meta;
use crate::codex_state::CodexThreadRef;
use crate::model::{AgentState, AgentType, PreviewTurn, SessionCacheState};
use crate::session_cache::SessionCacheSnapshot;
use crate::sidebar::model::{SidebarFolder, SidebarThread};
use crate::thread_meta::ThreadMeta;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

mod activity {
    use super::*;
    include!("tests/activity.rs");
}

mod history {
    use super::*;
    include!("tests/history.rs");
}

mod meta {
    use super::*;
    include!("tests/meta.rs");
}

mod support {
    use super::*;
    include!("tests/support.rs");
}
