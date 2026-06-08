use super::overrides::apply_activity_override;
use super::sort::apply_sort_activity;
use crate::sidebar::model::{SidebarThread, ThreadActivityOverride};
use std::collections::HashMap;
use std::sync::Arc;

pub(in crate::sidebar::build) fn merge_or_insert_thread(
    threads: &mut Vec<Arc<SidebarThread>>,
    mut history_entry: SidebarThread,
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    startup_thread_sort_activity: &HashMap<String, i64>,
) {
    apply_activity_override(&mut history_entry, activity_overrides);
    apply_sort_activity(
        &mut history_entry,
        thread_sort_activity,
        startup_thread_sort_activity,
    );

    let Some(existing) = matching_existing_thread(threads, &history_entry) else {
        threads.push(Arc::new(history_entry));
        return;
    };

    merge_history_into_existing(existing, history_entry);
    apply_activity_override(existing, activity_overrides);
    apply_sort_activity(existing, thread_sort_activity, startup_thread_sort_activity);
}

fn matching_existing_thread<'a>(
    threads: &'a mut [Arc<SidebarThread>],
    history_entry: &SidebarThread,
) -> Option<&'a mut SidebarThread> {
    threads
        .iter_mut()
        .find(|existing| threads_match(existing, history_entry))
        .map(Arc::make_mut)
}

fn threads_match(existing: &SidebarThread, history_entry: &SidebarThread) -> bool {
    existing.agent_type == history_entry.agent_type
        && ((existing.session_id.is_some() && existing.session_id == history_entry.session_id)
            || (existing.transcript_path.is_some()
                && existing.transcript_path == history_entry.transcript_path))
}

fn merge_history_into_existing(existing: &mut SidebarThread, history_entry: SidebarThread) {
    existing.session_id = existing.session_id.clone().or(history_entry.session_id);
    existing.transcript_path = existing
        .transcript_path
        .clone()
        .or(history_entry.transcript_path.clone());
    if existing.session_provider_name.is_none() {
        existing.session_provider_name = history_entry.session_provider_name.clone();
    }
    existing.archived = existing.archived || history_entry.archived;
    if existing.title.trim().is_empty() || existing.title.starts_with('%') {
        existing.title = history_entry.title;
    }
    if existing.upstream_title.is_none() {
        existing.upstream_title = history_entry.upstream_title;
    }
    if existing.subtitle.is_none() {
        existing.subtitle = history_entry
            .last_user_prompt
            .clone()
            .or(history_entry.subtitle.clone());
    }
    if existing.title_override.is_none() {
        existing.title_override = history_entry.title_override;
    }
    if existing.note.is_none() {
        existing.note = history_entry.note;
    }
    if existing.share_url.is_none() {
        existing.share_url = history_entry.share_url.clone();
    }
    if existing.cost.is_none() {
        existing.cost = history_entry.cost.clone();
    }
    if existing.token_summary.is_none() {
        existing.token_summary = history_entry.token_summary.clone();
    }
    merge_tags(existing, history_entry.tags);
    existing.pinned |= history_entry.pinned;
    existing.updated_at = existing.updated_at.max(history_entry.updated_at);
    existing.sort_updated_at = existing.sort_updated_at.max(history_entry.sort_updated_at);
    if existing.runtime_source.is_none() {
        existing.runtime_source = history_entry.runtime_source;
    }
    if existing.cached_preview_turns.is_empty() {
        existing.cached_preview_turns = history_entry.cached_preview_turns.clone();
    }
    if existing.session_cache_state.is_none() {
        existing.session_cache_state = history_entry.session_cache_state;
    }
    if existing.last_user_prompt.is_none() {
        existing.last_user_prompt = history_entry.last_user_prompt.clone();
    }
    if existing.last_assistant_message.is_none() {
        existing.last_assistant_message = history_entry.last_assistant_message.clone();
    }
}

fn merge_tags(existing: &mut SidebarThread, tags: Vec<String>) {
    if existing.tags.is_empty() {
        existing.tags = tags;
        return;
    }

    for tag in tags {
        if !existing
            .tags
            .iter()
            .any(|existing_tag| existing_tag == &tag)
        {
            existing.tags.push(tag);
        }
    }
}
