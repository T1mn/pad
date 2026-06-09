use super::meta::apply_thread_meta;
use crate::sidebar::model::SidebarFolder;
use std::collections::HashMap;
use std::sync::Arc;

mod thread;

use thread::thread_for_deleted_meta;

pub(super) fn build_trash_folders() -> Vec<SidebarFolder> {
    let mut folders: HashMap<String, SidebarFolder> = HashMap::new();
    let deleted = match crate::thread_meta::load_deleted_thread_meta() {
        Ok(deleted) => deleted,
        Err(err) => {
            crate::log_debug!("trash: failed to load deleted thread metadata: {}", err);
            return Vec::new();
        }
    };

    for (key, meta) in deleted {
        let Some(mut thread) = thread_for_deleted_meta(&key) else {
            crate::log_debug!(
                "trash: skipped unresolved deleted thread agent={} id={}",
                key.agent_type,
                key.thread_id
            );
            continue;
        };
        apply_thread_meta(&mut thread, &meta);

        let folder = folders
            .entry(thread.folder_key.clone())
            .or_insert_with(|| SidebarFolder {
                key: thread.folder_key.clone(),
                path: thread.working_dir.clone(),
                label: thread.folder_label.clone(),
                updated_at: 0,
                threads: Vec::new(),
            });
        folder.threads.push(Arc::new(thread));
    }

    let mut values = folders.into_values().collect::<Vec<_>>();
    for folder in &mut values {
        folder.threads.sort_by(crate::sidebar::thread_sort_key);
        folder.updated_at = folder
            .threads
            .first()
            .map(|thread| thread.sort_timestamp())
            .unwrap_or_default();
    }
    values.sort_by(crate::sidebar::folder_sort_key);
    values
}
