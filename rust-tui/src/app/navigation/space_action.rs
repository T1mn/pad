use super::super::App;
use crate::app::state::sidebar::{PendingSidebarSpaceAction, PendingSidebarSpaceActionKind};
use crate::sidebar::SidebarItem;
use std::time::{Duration, Instant};

impl App {
    pub fn queue_pending_sidebar_space_action(&mut self, window: Duration) -> bool {
        if self.sidebar.show_tree || self.preview_is_focused() {
            return false;
        }

        let kind = match self.selected_sidebar_item() {
            Some(SidebarItem::Folder(folder)) => {
                PendingSidebarSpaceActionKind::ToggleFolder(folder.key.clone())
            }
            Some(SidebarItem::Thread(thread)) => {
                PendingSidebarSpaceActionKind::CollapseParentFolder(thread.folder_key.clone())
            }
            None => return false,
        };

        self.sidebar.pending_space_action = Some(PendingSidebarSpaceAction {
            kind,
            deadline: Instant::now() + window,
        });
        true
    }

    pub fn pending_sidebar_space_action_is_active(&self) -> bool {
        self.sidebar
            .pending_space_action
            .as_ref()
            .map(|action| action.deadline > Instant::now())
            .unwrap_or(false)
    }

    pub fn clear_pending_sidebar_space_action(&mut self) {
        self.sidebar.pending_space_action = None;
    }

    pub fn flush_pending_sidebar_space_action_if_due(&mut self) -> bool {
        if self
            .sidebar
            .pending_space_action
            .as_ref()
            .map(|action| action.deadline <= Instant::now())
            .unwrap_or(false)
        {
            return self.flush_pending_sidebar_space_action();
        }

        false
    }

    pub fn flush_pending_sidebar_space_action(&mut self) -> bool {
        let Some(action) = self.sidebar.pending_space_action.take() else {
            return false;
        };

        match action.kind {
            PendingSidebarSpaceActionKind::ToggleFolder(folder_key) => {
                let folder_exists = self
                    .sidebar_folders_ref()
                    .iter()
                    .any(|folder| folder.key == folder_key);
                if !folder_exists {
                    return false;
                }
                if self.sidebar.expanded_folders.contains(&folder_key) {
                    self.sidebar.expanded_folders.remove(&folder_key);
                } else {
                    self.sidebar.expanded_folders.insert(folder_key.clone());
                }
                self.sidebar.selected_sidebar_key = Some(folder_key);
                self.invalidate_sidebar_visible_cache();
                self.sync_sidebar_selection();
                self.invalidate_preview();
                self.dirty = true;
                true
            }
            PendingSidebarSpaceActionKind::CollapseParentFolder(folder_key) => {
                if !self.sidebar.expanded_folders.remove(&folder_key) {
                    return false;
                }
                self.sidebar.selected_sidebar_key = Some(folder_key);
                self.invalidate_sidebar_visible_cache();
                self.sync_sidebar_selection();
                self.focus_panel();
                self.invalidate_preview();
                self.dirty = true;
                true
            }
        }
    }
}
