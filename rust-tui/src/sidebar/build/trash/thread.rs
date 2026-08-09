mod claude {
    use crate::model::{AgentState, AgentType};
    use crate::sidebar::display::{best_thread_title, clean_title, folder_display_label};
    use crate::sidebar::model::SidebarThread;

    pub(super) fn deleted_claude_thread(thread_id: &str) -> Option<SidebarThread> {
        crate::claude_history::thread_for_id(thread_id)
            .ok()
            .flatten()
            .map(build_claude_history_thread)
    }

    fn build_claude_history_thread(
        thread: crate::claude_history::ClaudeThreadRef,
    ) -> SidebarThread {
        let folder_key = thread.cwd.to_string_lossy().to_string();
        SidebarThread {
            key: format!("claude:{}", thread.session_id),
            folder_key: folder_key.clone(),
            working_dir: folder_key.clone(),
            folder_label: folder_display_label(&folder_key),
            agent_type: AgentType::Claude,
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
}
mod codex {
    use super::super::super::history_codex::build_codex_history_entry;
    use crate::sidebar::display::folder_display_label;
    use crate::sidebar::model::{SidebarFolder, SidebarThread};

    pub(super) fn deleted_codex_thread(thread_id: &str) -> Option<SidebarThread> {
        crate::codex_state::thread_for_id(thread_id)
            .ok()
            .flatten()
            .and_then(|thread| {
                if crate::sidebar::search::is_subagent_source(thread.source.as_deref()) {
                    None
                } else {
                    Some(build_codex_history_thread(&thread))
                }
            })
    }

    fn build_codex_history_thread(thread: &crate::codex_state::CodexThreadRef) -> SidebarThread {
        let folder_key = thread.cwd.to_string_lossy().to_string();
        let folder = SidebarFolder {
            key: folder_key.clone(),
            path: folder_key.clone(),
            label: folder_display_label(&folder_key),
            updated_at: 0,
            threads: Vec::new(),
        };
        build_codex_history_entry(&folder, thread, None, false)
    }
}
mod gemini {
    use crate::model::{AgentState, AgentType};
    use crate::sidebar::display::{best_thread_title, clean_title, folder_display_label};
    use crate::sidebar::model::SidebarThread;

    pub(super) fn deleted_gemini_thread(thread_id: &str) -> Option<SidebarThread> {
        crate::gemini_history::thread_for_id(thread_id)
            .ok()
            .flatten()
            .filter(|thread| thread.kind != "subagent")
            .map(build_gemini_history_thread)
    }

    fn build_gemini_history_thread(
        thread: crate::gemini_history::GeminiThreadRef,
    ) -> SidebarThread {
        let folder_key = thread.cwd.to_string_lossy().to_string();
        SidebarThread {
            key: format!("gemini:{}", thread.session_id),
            folder_key: folder_key.clone(),
            working_dir: folder_key.clone(),
            folder_label: folder_display_label(&folder_key),
            agent_type: AgentType::Gemini,
            session_id: Some(thread.session_id.clone()),
            transcript_path: Some(thread.transcript_path.to_string_lossy().to_string()),
            session_provider_name: None,
            title: best_thread_title(thread.title.as_deref(), Some(thread.session_id.as_str())),
            upstream_title: thread.title.as_deref().and_then(clean_title),
            generated_title: None,
            subtitle: thread
                .subtitle
                .as_deref()
                .and_then(clean_title)
                .or_else(|| thread.last_user_message.as_deref().and_then(clean_title)),
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
            state: AgentState::Idle,
            is_active: false,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            last_user_prompt: thread.last_user_message.clone(),
            last_assistant_message: thread.last_assistant_message.clone(),
            has_unread_stop: false,
            archived: thread.archived,
            deleted: false,
        }
    }
}
mod opencode {
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
}

use crate::sidebar::model::SidebarThread;
use crate::thread_meta::ThreadMetaKey;

pub(super) fn thread_for_deleted_meta(key: &ThreadMetaKey) -> Option<SidebarThread> {
    match key.agent_type.as_str() {
        "codex" => codex::deleted_codex_thread(&key.thread_id),
        "claude" => claude::deleted_claude_thread(&key.thread_id),
        "gemini" => gemini::deleted_gemini_thread(&key.thread_id),
        "opencode" => opencode::deleted_opencode_thread(&key.thread_id),
        _ => None,
    }
}
