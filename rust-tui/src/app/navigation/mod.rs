use super::App;
use crate::sidebar::SidebarItem;

mod cache;
mod folders {
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
}
mod movement;
mod selection;
mod space_action {
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
}

#[cfg(test)]
mod tests;

impl App {
    pub(super) fn nth_visible_thread_sidebar_index(
        items: &[SidebarItem],
        target: usize,
    ) -> Option<usize> {
        let mut visible_threads = 0usize;
        for (index, item) in items.iter().enumerate() {
            if item.as_thread().is_none() {
                continue;
            }
            if visible_threads == target {
                return Some(index);
            }
            visible_threads += 1;
        }
        None
    }

    pub(super) fn sidebar_item_is_navigable(
        items: &[SidebarItem],
        index: usize,
        item: &SidebarItem,
    ) -> bool {
        match item {
            SidebarItem::Thread(_) => true,
            SidebarItem::Folder(folder) => items
                .get(index + 1)
                .and_then(SidebarItem::as_thread)
                .is_none_or(|thread| thread.folder_key != folder.key),
        }
    }

    pub(super) fn next_navigable_sidebar_index(
        items: &[SidebarItem],
        current: Option<usize>,
        forward: bool,
    ) -> Option<usize> {
        let mut first = None;
        let mut last = None;
        let mut next = None;
        let mut previous = None;

        for (index, item) in items.iter().enumerate() {
            if !Self::sidebar_item_is_navigable(items, index, item) {
                continue;
            }

            first.get_or_insert(index);
            last = Some(index);

            if let Some(current) = current {
                if index > current && next.is_none() {
                    next = Some(index);
                }
                if index < current {
                    previous = Some(index);
                }
            }
        }

        match current {
            Some(_) if forward => next.or(first),
            Some(_) => previous.or(last),
            None if forward => first,
            None => last,
        }
    }
}
