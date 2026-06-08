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
