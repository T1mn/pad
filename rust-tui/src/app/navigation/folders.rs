use super::super::App;
use crate::sidebar::SidebarItem;

impl App {
    pub fn toggle_selected_folder(&mut self) -> bool {
        let Some(item) = self.selected_sidebar_item() else {
            return false;
        };
        let Some(folder) = item.as_folder() else {
            return false;
        };
        if self.sidebar.expanded_folders.contains(&folder.key) {
            self.sidebar.expanded_folders.remove(&folder.key);
        } else {
            self.sidebar.expanded_folders.insert(folder.key.clone());
        }
        self.invalidate_sidebar_visible_cache();
        self.sync_sidebar_selection();
        self.invalidate_preview();
        self.dirty = true;
        true
    }

    pub fn toggle_all_sidebar_folders(&mut self) -> bool {
        let folder_keys = self
            .sidebar_folders_ref()
            .iter()
            .map(|folder| folder.key.clone())
            .collect::<Vec<_>>();
        if folder_keys.is_empty() {
            return false;
        }

        let collapse_all = folder_keys
            .iter()
            .any(|key| self.sidebar.expanded_folders.contains(key));

        if collapse_all {
            for key in &folder_keys {
                self.sidebar.expanded_folders.remove(key);
            }
            if let Some(SidebarItem::Thread(thread)) = self.selected_sidebar_item() {
                self.sidebar.selected_sidebar_key = Some(thread.folder_key.clone());
            }
        } else {
            for key in &folder_keys {
                self.sidebar.expanded_folders.insert(key.clone());
            }
        }

        self.invalidate_sidebar_visible_cache();
        self.sync_sidebar_selection();
        self.focus_panel();
        self.invalidate_preview();
        self.dirty = true;
        true
    }

    pub fn expand_selected_folder(&mut self) -> bool {
        let Some(item) = self.selected_sidebar_item() else {
            return false;
        };
        let Some(folder) = item.as_folder() else {
            return false;
        };
        if self.sidebar.expanded_folders.insert(folder.key.clone()) {
            self.invalidate_sidebar_visible_cache();
            self.sync_sidebar_selection();
            self.invalidate_preview();
            self.dirty = true;
        }
        true
    }

    pub fn collapse_selected_folder(&mut self) -> bool {
        let Some(item) = self.selected_sidebar_item() else {
            return false;
        };
        match item {
            SidebarItem::Folder(folder) => {
                if self.sidebar.expanded_folders.remove(&folder.key) {
                    self.invalidate_sidebar_visible_cache();
                    self.sync_sidebar_selection();
                    self.invalidate_preview();
                    self.dirty = true;
                }
                true
            }
            SidebarItem::Thread(thread) => {
                self.sidebar.selected_sidebar_key = Some(thread.folder_key.clone());
                self.sync_sidebar_selection();
                self.focus_panel();
                self.invalidate_preview();
                self.dirty = true;
                true
            }
        }
    }

    pub fn collapse_parent_folder_for_selected_thread(&mut self) -> bool {
        let Some(SidebarItem::Thread(thread)) = self.selected_sidebar_item() else {
            return false;
        };
        if !self.sidebar.expanded_folders.remove(&thread.folder_key) {
            return false;
        }
        self.sidebar.selected_sidebar_key = Some(thread.folder_key.clone());
        self.invalidate_sidebar_visible_cache();
        self.sync_sidebar_selection();
        self.focus_panel();
        self.invalidate_preview();
        self.dirty = true;
        true
    }
}
