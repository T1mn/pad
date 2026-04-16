use super::activity::merge_or_insert_thread;
use crate::claude_history::ClaudeThreadRef;
use crate::model::{AgentState, AgentType};
use std::collections::HashMap;
use std::path::Path;

use super::super::display::{best_thread_title, clean_title};
use super::super::model::{SidebarFolder, SidebarThread, ThreadActivityOverride};

pub(super) fn merge_claude_threads(
    folder: &mut SidebarFolder,
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    startup_thread_sort_activity: &HashMap<String, i64>,
    claude_threads: Option<&[ClaudeThreadRef]>,
    archived_threads_view: bool,
) -> usize {
    let Some(threads) = claude_threads else {
        return 0;
    };

    let mut merged = 0usize;
    for thread in threads
        .iter()
        .filter(|thread| thread_matches_folder(thread, &folder.path))
    {
        let sort_updated_at = initial_sort_updated_at(thread.updated_at, archived_threads_view);
        let history_entry = SidebarThread {
            key: format!("claude:{}", thread.session_id),
            folder_key: folder.key.clone(),
            working_dir: folder.path.clone(),
            folder_label: folder.label.clone(),
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
        merge_or_insert_thread(
            &mut folder.threads,
            history_entry,
            activity_overrides,
            thread_sort_activity,
            startup_thread_sort_activity,
        );
        merged += 1;
    }
    merged
}

fn thread_matches_folder(thread: &ClaudeThreadRef, folder_path: &str) -> bool {
    thread.cwd == Path::new(folder_path) || thread.cwd.to_string_lossy() == folder_path
}

fn initial_sort_updated_at(updated_at: i64, archived_threads_view: bool) -> i64 {
    if archived_threads_view {
        updated_at
    } else {
        0
    }
}
