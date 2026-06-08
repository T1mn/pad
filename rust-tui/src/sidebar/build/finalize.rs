use std::collections::HashMap;
use std::sync::Arc;

use super::super::model::SidebarFolder;
use super::super::sort::thread_sort_key;
use super::activity::apply_sort_activity;

pub(super) fn finalize_folder_threads(
    folder: &mut SidebarFolder,
    thread_sort_activity: &HashMap<String, i64>,
    startup_thread_sort_activity: &HashMap<String, i64>,
    retain_deleted_with_live_pane_only: bool,
) {
    if retain_deleted_with_live_pane_only {
        folder
            .threads
            .retain(|thread| !thread.deleted || thread.live_pane_id.is_some());
    }
    for thread in &mut folder.threads {
        apply_sort_activity(
            Arc::make_mut(thread),
            thread_sort_activity,
            startup_thread_sort_activity,
        );
    }
    folder.threads.sort_by(thread_sort_key);
    folder.updated_at = folder
        .threads
        .first()
        .map(|thread| thread.sort_timestamp())
        .unwrap_or_default();
}
