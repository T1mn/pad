use super::history_codex::build_codex_history_entry;
use super::meta::apply_thread_meta;
use crate::model::{AgentState, AgentType};
use crate::sidebar::display::{best_thread_title, clean_title, folder_display_label};
use crate::sidebar::model::{SidebarFolder, SidebarThread};
use std::collections::HashMap;
use std::sync::Arc;

pub(super) fn build_trash_folders() -> Vec<SidebarFolder> {
    let mut folders: HashMap<String, SidebarFolder> = HashMap::new();
    let deleted = match crate::thread_meta::load_deleted_thread_meta() {
        Ok(deleted) => deleted,
        Err(err) => {
            crate::log_debug!("trash: failed to load deleted thread metadata: {}", err);
            return Vec::new();
        }
    };

    for (key, meta) in deleted {
        let maybe_thread = match key.agent_type.as_str() {
            "codex" => crate::codex_state::thread_for_id(&key.thread_id)
                .ok()
                .flatten()
                .and_then(|thread| {
                    if crate::sidebar::search::is_subagent_source(thread.source.as_deref()) {
                        None
                    } else {
                        Some(build_codex_history_thread(&thread))
                    }
                }),
            "claude" => crate::claude_history::thread_for_id(&key.thread_id)
                .ok()
                .flatten()
                .map(build_claude_history_thread),
            "gemini" => crate::gemini_history::thread_for_id(&key.thread_id)
                .ok()
                .flatten()
                .filter(|thread| thread.kind != "subagent")
                .map(build_gemini_history_thread),
            _ => None,
        };

        let Some(mut thread) = maybe_thread else {
            crate::log_debug!(
                "trash: skipped unresolved deleted thread agent={} id={}",
                key.agent_type,
                key.thread_id
            );
            continue;
        };
        apply_thread_meta(&mut thread, &meta);

        let folder = folders
            .entry(thread.folder_key.clone())
            .or_insert_with(|| SidebarFolder {
                key: thread.folder_key.clone(),
                path: thread.working_dir.clone(),
                label: thread.folder_label.clone(),
                updated_at: 0,
                threads: Vec::new(),
            });
        folder.threads.push(Arc::new(thread));
    }

    let mut values = folders
        .into_values()
        .filter(|folder| !folder.threads.is_empty())
        .collect::<Vec<_>>();
    for folder in &mut values {
        folder.threads.sort_by(crate::sidebar::thread_sort_key);
        folder.updated_at = folder
            .threads
            .first()
            .map(|thread| thread.sort_timestamp())
            .unwrap_or_default();
    }
    values.sort_by(crate::sidebar::folder_sort_key);
    values
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
        title: best_thread_title(thread.title.as_deref(), Some(thread.session_id.as_str())),
        upstream_title: thread.title.as_deref().and_then(clean_title),
        generated_title: None,
        subtitle: None,
        title_override: None,
        note: None,
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

fn build_gemini_history_thread(thread: crate::gemini_history::GeminiThreadRef) -> SidebarThread {
    let folder_key = thread.cwd.to_string_lossy().to_string();
    SidebarThread {
        key: format!("gemini:{}", thread.session_id),
        folder_key: folder_key.clone(),
        working_dir: folder_key.clone(),
        folder_label: folder_display_label(&folder_key),
        agent_type: AgentType::Gemini,
        runtime_source: None,
        session_id: Some(thread.session_id.clone()),
        transcript_path: Some(thread.transcript_path.to_string_lossy().to_string()),
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
        last_user_prompt: thread.last_user_message.clone(),
        last_assistant_message: thread.last_assistant_message.clone(),
        has_unread_stop: false,
        archived: thread.archived,
        deleted: false,
    }
}
