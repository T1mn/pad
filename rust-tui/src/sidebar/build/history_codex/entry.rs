use super::snapshot::apply_session_cache_snapshot;
use crate::codex_state::CodexThreadRef;
use crate::model::{AgentState, AgentType};
use crate::session_cache::SessionCacheSnapshot;
use crate::sidebar::display::{best_thread_title, clean_title};
use crate::sidebar::model::{SidebarFolder, SidebarThread};

pub(in crate::sidebar::build) fn build_codex_history_entry(
    folder: &SidebarFolder,
    thread: &CodexThreadRef,
    snapshot: Option<&SessionCacheSnapshot>,
    archived_threads_view: bool,
) -> SidebarThread {
    let title = best_thread_title(thread.title.as_deref(), Some(thread.thread_id.as_str()));
    let sort_updated_at = initial_sort_updated_at(thread.updated_at, archived_threads_view);
    let mut history_entry = SidebarThread {
        key: format!("codex:{}", thread.thread_id),
        folder_key: folder.key.clone(),
        working_dir: folder.path.clone(),
        folder_label: folder.label.clone(),
        agent_type: AgentType::Codex,
        runtime_source: None,
        session_id: Some(thread.thread_id.clone()),
        transcript_path: Some(thread.rollout_path.to_string_lossy().to_string()),
        session_provider_name: crate::sidebar::provider::resolve_session_provider_name(
            &AgentType::Codex,
            Some(thread.rollout_path.as_path()),
        ),
        title,
        upstream_title: thread.title.as_deref().and_then(clean_title),
        generated_title: None,
        subtitle: None,
        title_override: None,
        note: None,
        share_url: None,
        cost: None,
        token_summary: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: thread.updated_at,
        sort_updated_at,
        live_pane_id: None,
        live_location: None,
        pid: None,
        git_info: None,
        state: AgentState::Idle,
        is_active: false,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: thread.archived,
        deleted: false,
    };

    if let Some(snapshot) = snapshot {
        apply_session_cache_snapshot(&mut history_entry, snapshot);
    }

    if history_entry.subtitle.is_none() {
        history_entry.subtitle = thread.first_user_message.as_deref().and_then(clean_title);
    }

    history_entry
}

fn initial_sort_updated_at(updated_at: i64, archived_threads_view: bool) -> i64 {
    if archived_threads_view {
        updated_at
    } else {
        0
    }
}
