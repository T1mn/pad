use super::entry::build_codex_history_entry;
use crate::session_cache::SessionCacheSnapshot;
use crate::sidebar::build::activity::merge_or_insert_thread;
use crate::sidebar::model::{SidebarFolder, ThreadActivityOverride};
use std::collections::HashMap;
use std::path::Path;

pub(in crate::sidebar::build) fn merge_codex_threads(
    folder: &mut SidebarFolder,
    activity_overrides: &[ThreadActivityOverride],
    thread_sort_activity: &HashMap<String, i64>,
    startup_thread_sort_activity: &HashMap<String, i64>,
    codex_session_snapshots: &HashMap<String, SessionCacheSnapshot>,
    archived_threads_view: bool,
) -> usize {
    let Ok(threads) = codex_threads_for_folder(folder, archived_threads_view) else {
        return 0;
    };

    let mut merged = 0usize;
    for thread in threads {
        if crate::sidebar::search::is_subagent_source(thread.source.as_deref()) {
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

fn codex_threads_for_folder(
    folder: &SidebarFolder,
    archived_threads_view: bool,
) -> std::io::Result<Vec<crate::codex_state::CodexThreadRef>> {
    if archived_threads_view {
        crate::codex_state::archived_threads_for_cwd(Path::new(&folder.path))
    } else {
        crate::codex_state::threads_for_cwd(Path::new(&folder.path))
    }
}
