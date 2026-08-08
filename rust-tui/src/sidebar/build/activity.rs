mod merge;
mod overrides {
    use crate::sidebar::model::{SidebarThread, ThreadActivityOverride};

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
}
mod sort {
    use crate::sidebar::model::SidebarThread;
    use std::collections::HashMap;

    pub(in crate::sidebar::build) fn apply_sort_activity(
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
}

pub(super) use merge::merge_or_insert_thread;
pub(super) use sort::apply_sort_activity;
