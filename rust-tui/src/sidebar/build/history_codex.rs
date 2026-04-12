use super::activity::merge_or_insert_thread;
use crate::codex_state::CodexThreadRef;
use crate::model::{AgentState, AgentType};
use crate::session_cache::SessionCacheSnapshot;
use std::collections::HashMap;
use std::path::Path;

use super::super::display::{best_thread_title, clean_title};
use super::super::model::{SidebarFolder, SidebarThread, ThreadActivityOverride};

pub(super) fn merge_codex_threads(
    folder: &mut SidebarFolder,
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    startup_thread_sort_activity: &HashMap<String, i64>,
    codex_session_snapshots: &HashMap<String, SessionCacheSnapshot>,
    archived_threads_view: bool,
) -> usize {
    let threads = if archived_threads_view {
        crate::codex_state::archived_threads_for_cwd(Path::new(&folder.path))
    } else {
        crate::codex_state::threads_for_cwd(Path::new(&folder.path))
    };
    let Ok(threads) = threads else {
        return 0;
    };

    let mut merged = 0usize;
    for thread in threads {
        if super::super::search::is_subagent_source(thread.source.as_deref()) {
            continue;
        }
        let history_entry = build_codex_history_entry(
            folder,
            &thread,
            codex_session_snapshots.get(&thread.thread_id),
            archived_threads_view,
        );

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

pub(super) fn build_codex_history_entry(
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
        title,
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

    if let Some(snapshot) = snapshot {
        apply_session_cache_snapshot(&mut history_entry, snapshot);
    }

    if history_entry.subtitle.is_none() {
        history_entry.subtitle = thread.first_user_message.as_deref().and_then(clean_title);
    }

    history_entry
}

fn apply_session_cache_snapshot(thread: &mut SidebarThread, snapshot: &SessionCacheSnapshot) {
    if thread.transcript_path.is_none() {
        thread.transcript_path = snapshot.transcript_path.clone();
    }

    if !snapshot.recent_turns.is_empty() {
        thread.cached_preview_turns = snapshot.recent_turns.clone();
    }

    if let Some(prompt) = snapshot
        .last_user_prompt
        .as_deref()
        .and_then(clean_cached_prompt)
        .or_else(|| {
            snapshot
                .recent_turns
                .first()
                .and_then(|turn| clean_cached_prompt(&turn.question))
        })
    {
        thread.last_user_prompt = Some(prompt.clone());
        thread.subtitle = Some(prompt);
    }

    if let Some(answer) = snapshot
        .last_assistant_message
        .as_deref()
        .and_then(clean_title)
    {
        thread.last_assistant_message = Some(answer);
    }

    thread.session_cache_state = Some(snapshot.state);
}

fn clean_cached_prompt(text: &str) -> Option<String> {
    clean_title(&crate::preview_source::codex::normalize_codex_user_text(
        text, None,
    ))
}

fn initial_sort_updated_at(updated_at: i64, archived_threads_view: bool) -> i64 {
    if archived_threads_view {
        updated_at
    } else {
        0
    }
}
