use super::super::meta::apply_thread_metadata;
use super::thread_from_live_panel;
use crate::model::AgentPanel;
use std::collections::HashMap;
use std::sync::Arc;

use super::super::super::display::folder_display_label;
use super::super::super::model::SidebarFolder;
use super::super::super::sort::{folder_sort_key, thread_sort_key};

pub(in crate::sidebar::build) fn build_live_panel_fallback_folders(
    panels: &[AgentPanel],
) -> Vec<SidebarFolder> {
    let mut folders: HashMap<String, SidebarFolder> = HashMap::new();

    for panel in panels {
        let folder_key = panel.working_dir.clone();
        let folder = folders
            .entry(folder_key.clone())
            .or_insert_with(|| SidebarFolder {
                key: folder_key.clone(),
                path: panel.working_dir.clone(),
                label: folder_display_label(&panel.working_dir),
                updated_at: 0,
                threads: Vec::new(),
            });
        folder.threads.push(Arc::new(thread_from_live_panel(panel)));
    }

    apply_thread_metadata(&mut folders);

    let mut values = folders
        .into_values()
        .filter(|folder| !folder.threads.is_empty())
        .collect::<Vec<_>>();
    for folder in &mut values {
        folder.threads.sort_by(thread_sort_key);
        folder.updated_at = folder
            .threads
            .first()
            .map(|thread| thread.sort_timestamp())
            .unwrap_or_default();
    }
    values.sort_by(folder_sort_key);
    values
}
