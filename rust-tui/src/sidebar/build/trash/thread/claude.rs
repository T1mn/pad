use crate::model::{AgentState, AgentType};
use crate::sidebar::display::{best_thread_title, clean_title, folder_display_label};
use crate::sidebar::model::SidebarThread;

pub(super) fn deleted_claude_thread(thread_id: &str) -> Option<SidebarThread> {
    crate::claude_history::thread_for_id(thread_id)
        .ok()
        .flatten()
        .map(build_claude_history_thread)
}

fn build_claude_history_thread(thread: crate::claude_history::ClaudeThreadRef) -> SidebarThread {
    let folder_key = thread.cwd.to_string_lossy().to_string();
    SidebarThread {
        key: format!("claude:{}", thread.session_id),
        folder_key: folder_key.clone(),
        working_dir: folder_key.clone(),
        folder_label: folder_display_label(&folder_key),
        agent_type: AgentType::Claude,
        runtime_source: None,
        session_id: Some(thread.session_id.clone()),
        transcript_path: Some(thread.transcript_path.to_string_lossy().to_string()),
        session_provider_name: None,
        title: best_thread_title(thread.title.as_deref(), Some(thread.session_id.as_str())),
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
        sort_updated_at: thread.updated_at,
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
    }
}
