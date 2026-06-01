use crate::sidebar::model::{SidebarThread, ThreadActivityOverride};
use std::collections::HashMap;
use std::sync::Arc;

pub(super) fn merge_or_insert_thread(
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
    if let Some(existing) = threads.iter_mut().find(|existing| {
        existing.agent_type == history_entry.agent_type
            && ((existing.session_id.is_some() && existing.session_id == history_entry.session_id)
                || (existing.transcript_path.is_some()
                    && existing.transcript_path == history_entry.transcript_path))
    }) {
        let existing = Arc::make_mut(existing);
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
        if existing.tags.is_empty() {
            existing.tags = history_entry.tags;
        } else {
            for tag in history_entry.tags {
                if !existing
                    .tags
                    .iter()
                    .any(|existing_tag| existing_tag == &tag)
                {
                    existing.tags.push(tag);
                }
            }
        }
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
        apply_activity_override(existing, activity_overrides);
        apply_sort_activity(existing, thread_sort_activity, startup_thread_sort_activity);
        return;
    }

    threads.push(Arc::new(history_entry));
}

pub(super) fn apply_sort_activity(
    thread: &mut SidebarThread,
    thread_sort_activity: &HashMap<String, i64>,
    startup_thread_sort_activity: &HashMap<String, i64>,
) {
    let activity_keys = thread.sort_activity_keys();
    if let Some(sort_updated_at) = activity_keys
        .iter()
        .find_map(|key| thread_sort_activity.get(key).copied())
        .or_else(|| {
            activity_keys
                .iter()
                .find_map(|key| startup_thread_sort_activity.get(key).copied())
        })
    {
        thread.sort_updated_at = thread.sort_updated_at.max(sort_updated_at);
    }
}

pub(super) fn apply_activity_override(
    thread: &mut SidebarThread,
    activity_overrides: &[ThreadActivityOverride],
) {
    let Some(override_entry) = activity_overrides.iter().find(|entry| {
        entry.agent_type == thread.agent_type
            && ((entry.session_id.is_some() && entry.session_id == thread.session_id)
                || (entry.transcript_path.is_some()
                    && entry.transcript_path == thread.transcript_path))
    }) else {
        return;
    };

    thread.state = override_entry.state.clone();
    thread.is_active = override_entry.is_active;
    thread.updated_at = thread.updated_at.max(override_entry.updated_at);
    if thread.last_user_prompt.is_none() {
        thread.last_user_prompt = override_entry.last_user_prompt.clone();
    }
    if thread.subtitle.is_none() {
        thread.subtitle = override_entry.last_user_prompt.clone();
    }
    if thread.last_assistant_message.is_none() {
        thread.last_assistant_message = override_entry.last_assistant_message.clone();
    }
    if thread.title.trim().is_empty() || thread.title == "untitled" {
        thread.title = thread
            .upstream_title
            .clone()
            .or_else(|| thread.session_id.clone())
            .unwrap_or_else(|| "untitled".to_string());
    }
}
