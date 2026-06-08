use crate::model::{AgentState, AgentType};
use crate::sidebar::display::{best_thread_title, clean_title, folder_display_label};
use crate::sidebar::model::SidebarThread;

pub(super) fn deleted_opencode_thread(thread_id: &str) -> Option<SidebarThread> {
    crate::opencode_history::thread_for_id(thread_id)
        .ok()
        .flatten()
        .map(build_opencode_history_thread)
}

fn build_opencode_history_thread(
    thread: crate::opencode_history::OpenCodeThreadRef,
) -> SidebarThread {
    SidebarThread {
        key: format!("opencode:{}", thread.session_id),
        folder_key: thread.cwd.to_string_lossy().to_string(),
        working_dir: thread.cwd.to_string_lossy().to_string(),
        folder_label: folder_display_label(&thread.cwd.to_string_lossy()),
        agent_type: AgentType::OpenCode,
        runtime_source: None,
        session_id: Some(thread.session_id.clone()),
        transcript_path: Some(thread.db_path.to_string_lossy().to_string()),
        session_provider_name: thread.provider_name.clone(),
        title: best_thread_title(thread.title.as_deref(), Some(thread.session_id.as_str())),
        upstream_title: thread.title.as_deref().and_then(clean_title),
        generated_title: None,
        subtitle: thread.last_user_message.as_deref().and_then(clean_title),
        title_override: None,
        note: None,
        share_url: thread.share_url.clone(),
        cost: thread.cost.clone(),
        token_summary: thread.token_summary.clone(),
        tags: Vec::new(),
        pinned: false,
        updated_at: thread.updated_at,
        sort_updated_at: thread.updated_at,
        live_pane_id: None,
        live_location: None,
        pid: None,
        git_info: None,
        state: AgentState::Idle,
        is_active: false,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        last_user_prompt: thread.last_user_message,
        last_assistant_message: thread.last_assistant_message,
        has_unread_stop: false,
        archived: thread.archived,
        deleted: false,
    }
}
