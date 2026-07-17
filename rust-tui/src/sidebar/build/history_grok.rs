use super::activity::merge_or_insert_thread;
use crate::grok_history::GrokThreadRef;
use crate::model::{AgentState, AgentType};
use std::collections::HashMap;
use std::path::Path;

use super::super::display::{best_thread_title, clean_title};
use super::super::model::{SidebarFolder, SidebarThread, ThreadActivityOverride};

pub(super) fn merge_grok_threads(
    folder: &mut SidebarFolder,
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    startup_thread_sort_activity: &HashMap<String, i64>,
    grok_threads: Option<&[GrokThreadRef]>,
    archived_threads_view: bool,
) -> usize {
    if archived_threads_view {
        return 0;
    }
    let Some(threads) = grok_threads else {
        return 0;
    };
    let mut merged = 0;
    for thread in threads
        .iter()
        .filter(|thread| thread.cwd == Path::new(&folder.path))
    {
        let history_entry = SidebarThread {
            key: format!("grok:{}", thread.session_id),
            folder_key: folder.key.clone(),
            working_dir: folder.path.clone(),
            folder_label: folder.label.clone(),
            agent_type: AgentType::Grok,
            session_id: Some(thread.session_id.clone()),
            transcript_path: Some(thread.transcript_path.to_string_lossy().to_string()),
            session_provider_name: None,
            title: best_thread_title(thread.title.as_deref(), Some(&thread.session_id)),
            upstream_title: thread.title.as_deref().and_then(clean_title),
            generated_title: None,
            subtitle: thread.model_name.as_deref().and_then(clean_title),
            title_override: None,
            note: None,
            share_url: None,
            cost: None,
            token_summary: None,
            tags: Vec::new(),
            pinned: false,
            updated_at: thread.updated_at,
            sort_updated_at: 0,
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
            archived: false,
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
