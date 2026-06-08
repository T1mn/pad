use super::super::super::history_codex::build_codex_history_entry;
use crate::sidebar::display::folder_display_label;
use crate::sidebar::model::{SidebarFolder, SidebarThread};

pub(super) fn deleted_codex_thread(thread_id: &str) -> Option<SidebarThread> {
    crate::codex_state::thread_for_id(thread_id)
        .ok()
        .flatten()
        .and_then(|thread| {
            if crate::sidebar::search::is_subagent_source(thread.source.as_deref()) {
                None
            } else {
                Some(build_codex_history_thread(&thread))
            }
        })
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
