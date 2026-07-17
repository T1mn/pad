use crate::claude_history::ClaudeThreadRef;
use crate::gemini_history::GeminiThreadRef;
use crate::grok_history::GrokThreadRef;
use crate::model::AgentType;
use crate::opencode_history::OpenCodeThreadRef;
use crate::session_cache::SessionCacheSnapshot;
use std::collections::HashMap;

pub(super) struct HistorySources {
    pub(super) codex_session_snapshots: HashMap<String, SessionCacheSnapshot>,
    pub(super) claude_threads: Option<Vec<ClaudeThreadRef>>,
    pub(super) gemini_threads: Option<Vec<GeminiThreadRef>>,
    pub(super) grok_threads: Option<Vec<GrokThreadRef>>,
    pub(super) opencode_threads: Option<Vec<OpenCodeThreadRef>>,
}

pub(super) fn load_history_sources(live_only: bool, archived_threads_view: bool) -> HistorySources {
    HistorySources {
        codex_session_snapshots: load_codex_session_snapshots(live_only, archived_threads_view),
        claude_threads: load_claude_threads(live_only, archived_threads_view),
        gemini_threads: load_gemini_threads(live_only, archived_threads_view),
        grok_threads: load_grok_threads(live_only, archived_threads_view),
        opencode_threads: load_opencode_threads(live_only, archived_threads_view),
    }
}

fn load_grok_threads(live_only: bool, archived_threads_view: bool) -> Option<Vec<GrokThreadRef>> {
    if live_only || archived_threads_view {
        None
    } else {
        crate::grok_history::all_threads().ok()
    }
}

fn load_codex_session_snapshots(
    live_only: bool,
    archived_threads_view: bool,
) -> HashMap<String, SessionCacheSnapshot> {
    if !live_only || archived_threads_view {
        crate::session_cache::load_snapshots_by_agent_type(&AgentType::Codex)
    } else {
        HashMap::new()
    }
}

fn load_claude_threads(
    live_only: bool,
    archived_threads_view: bool,
) -> Option<Vec<ClaudeThreadRef>> {
    if archived_threads_view {
        crate::claude_history::all_archived_threads().ok()
    } else if live_only {
        None
    } else {
        crate::claude_history::all_threads().ok()
    }
}

fn load_gemini_threads(
    live_only: bool,
    archived_threads_view: bool,
) -> Option<Vec<GeminiThreadRef>> {
    if archived_threads_view {
        crate::gemini_history::all_archived_threads().ok()
    } else if live_only {
        None
    } else {
        crate::gemini_history::all_threads().ok()
    }
}

fn load_opencode_threads(
    live_only: bool,
    archived_threads_view: bool,
) -> Option<Vec<OpenCodeThreadRef>> {
    if archived_threads_view {
        crate::opencode_history::all_archived_threads().ok()
    } else if live_only {
        None
    } else {
        crate::opencode_history::all_threads().ok()
    }
}
