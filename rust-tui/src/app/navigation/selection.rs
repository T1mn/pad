use super::super::App;
use crate::app::state::sidebar::ThreadListView;
use crate::model::AgentPanel;
use crate::sidebar::{SidebarFolder, SidebarItem, SidebarThread};
use std::path::PathBuf;

impl App {
    pub fn sync_sidebar_selection(&mut self) {
        self.ensure_visible_sidebar_items_cache();

        if self.sidebar.visible_sidebar_items_cache.is_empty() {
            self.sidebar.selected_sidebar_key = None;
            self.table_state.select(None);
            return;
        }

        let pending_sidebar_selection_index = self.sidebar.pending_sidebar_selection_index.take();
        let mut selected_key = self.sidebar.selected_sidebar_key.clone();

        let selected_index = {
            let folders = &self.sidebar.sidebar_folders_cache;
            let items = &self.sidebar.visible_sidebar_items_cache;

            let mut selected_index = selected_key
                .as_deref()
                .and_then(|key| items.iter().position(|item| item.key() == key));

            if selected_index.is_none() {
                if let Some(folder_key) = selected_key.as_deref().and_then(|key| {
                    folders.iter().find_map(|folder| {
                        if folder.key == key
                            || folder.threads.iter().any(|thread| thread.key == key)
                        {
                            Some(folder.key.as_str())
                        } else {
                            None
                        }
                    })
                }) {
                    selected_index = items.iter().position(|item| item.key() == folder_key);
                    if selected_index.is_some() {
                        selected_key = Some(folder_key.to_string());
                    }
                }
            }

            if selected_index.is_none() {
                if let Some(preferred_index) = pending_sidebar_selection_index {
                    let clamped_index = preferred_index.min(items.len().saturating_sub(1));
                    selected_index = Some(clamped_index);
                    selected_key = items.get(clamped_index).map(|item| item.key().to_string());
                }
            }

            if selected_index.is_none() {
                selected_key = Some(items[0].key().to_string());
                Some(0)
            } else {
                selected_index
            }
        };

        self.sidebar.selected_sidebar_key = selected_key;
        self.table_state.select(selected_index);
    }

    pub fn selected_sidebar_item(&mut self) -> Option<SidebarItem> {
        let selected_key = self.sidebar.selected_sidebar_key.clone();
        let selected_index = self.table_state.selected();
        let items = self.visible_sidebar_items_ref();
        if items.is_empty() {
            return None;
        }

        if let Some(key) = selected_key.as_deref() {
            if let Some(item) = items.iter().find(|item| item.key() == key) {
                return Some(item.clone());
            }
        }

        selected_index
            .and_then(|index| items.get(index).cloned())
            .or_else(|| items.first().cloned())
    }

    pub fn selected_preview_thread(&mut self) -> Option<SidebarThread> {
        if self.sidebar.selected_sidebar_key.is_none()
            && self.thread_list_view() == ThreadListView::Normal
        {
            if let Some(panel) = self
                .table_state
                .selected()
                .and_then(|index| self.panels.get(index))
            {
                let mut thread = crate::sidebar::thread_from_live_panel(panel);
                self.apply_cached_preview_to_thread(&mut thread);
                return Some(thread);
            }
        }

        let mut thread = match self.selected_sidebar_item()? {
            SidebarItem::Folder(folder) => self
                .sidebar_folders_ref()
                .iter()
                .find(|candidate| candidate.key == folder.key)
                .and_then(SidebarFolder::primary_thread)?,
            SidebarItem::Thread(thread) => thread.as_ref().clone(),
        };
        self.apply_cached_preview_to_thread(&mut thread);
        Some(thread)
    }

    pub fn select_sidebar_index(&mut self, index: usize, invalidate_preview: bool) -> bool {
        let visible_len = self.visible_sidebar_items_ref().len();
        if index >= visible_len {
            return false;
        }

        let selected_key = self
            .visible_sidebar_items_ref()
            .get(index)
            .map(|item| item.key().to_string());
        self.table_state.select(Some(index));
        self.sidebar.selected_sidebar_key = selected_key;
        self.clear_unread_stop_for_selected_panel();
        if invalidate_preview {
            self.invalidate_preview();
        }
        self.update_tree_for_selection();
        self.dirty = true;
        true
    }

    #[allow(dead_code)]
    pub fn filtered_panels(&self) -> Vec<&AgentPanel> {
        if self.search_query.is_empty() {
            self.panels.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.panels
                .iter()
                .filter(|p| {
                    p.session.to_lowercase().contains(&query)
                        || p.window.to_lowercase().contains(&query)
                        || p.working_dir.to_lowercase().contains(&query)
                })
                .collect()
        }
    }

    pub fn selected_panel(&mut self) -> Option<&AgentPanel> {
        let pane_id = self
            .selected_preview_thread()
            .and_then(|thread| thread.live_pane_id)?;
        self.panels.iter().find(|panel| panel.pane_id == pane_id)
    }

    pub fn update_tree_for_selection(&mut self) {
        if self.sidebar.show_tree {
            if let Some(thread) = self.selected_preview_thread() {
                let path = PathBuf::from(&thread.working_dir);
                if path.exists() {
                    let should_update = match &self.sidebar.file_tree {
                        None => true,
                        Some(tree) => tree.root_path != path,
                    };
                    if should_update {
                        self.sidebar.file_tree = Some(crate::tree::FileTree::new(path));
                        self.dirty = true;
                    }
                }
            }
        }
    }
}
