use crate::model::AgentType;
use crate::thread_meta::{ThreadMeta, ThreadMetaKey};
use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Arc;

use super::super::display::clean_title;
use super::super::model::{SidebarFolder, SidebarThread};

pub(super) fn apply_thread_metadata(folders: &mut HashMap<String, SidebarFolder>) {
    let keys = collect_thread_meta_keys(folders);
    if keys.is_empty() {
        return;
    }

    let Ok(meta_map) = crate::thread_meta::load_thread_meta_batch(&keys) else {
        crate::log_debug!(
            "thread_meta: failed to load batch metadata for {} threads",
            keys.len()
        );
        return;
    };

    for folder in folders.values_mut() {
        for thread in &mut folder.threads {
            apply_thread_meta_lookup(Arc::make_mut(thread), &meta_map);
        }
    }
}

fn collect_thread_meta_keys(folders: &HashMap<String, SidebarFolder>) -> Vec<ThreadMetaKey> {
    let mut keys = Vec::new();
    let mut seen = HashSet::new();

    for folder in folders.values() {
        for thread in &folder.threads {
            let Some(session_id) = thread.session_id.as_deref() else {
                continue;
            };
            let key = ThreadMetaKey::new(thread.agent_type.to_string(), session_id);
            if seen.insert(key.clone()) {
                keys.push(key);
            }
        }
    }

    keys
}

fn apply_thread_meta_lookup(
    thread: &mut SidebarThread,
    meta_map: &HashMap<ThreadMetaKey, ThreadMeta>,
) {
    let Some(session_id) = thread.session_id.as_deref() else {
        return;
    };
    let key = ThreadMetaKey::new(thread.agent_type.to_string(), session_id);
    if let Some(meta) = meta_map.get(&key) {
        apply_thread_meta(thread, meta);
    }
}

pub(super) fn apply_thread_meta(thread: &mut SidebarThread, meta: &ThreadMeta) {
    thread.title_override = meta.title_override.clone();
    thread.note = meta.note.clone();
    thread.pinned = meta.pinned;
    thread.tags = meta.tags.clone();

    if let Some(override_title) = meta.title_override.as_deref().and_then(clean_title) {
        thread.title = override_title;
    }
}

pub(super) fn load_thread_meta_for_panel(
    agent_type: &AgentType,
    session_id: &str,
) -> io::Result<Option<ThreadMeta>> {
    crate::thread_meta::load_thread_meta(&agent_type.to_string(), session_id)
}
