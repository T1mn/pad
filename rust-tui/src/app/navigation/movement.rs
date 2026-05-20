use super::super::App;
use crate::log_debug;
use crate::sidebar::{SidebarItem, SidebarThread};

impl App {
    pub fn next(&mut self) {
        if self.sidebar.show_tree {
            if let Some(ref mut tree) = self.sidebar.file_tree {
                log_debug!("nav: next (tree) selected={:?}", tree.state.selected());
                tree.next();
                self.dirty = true;
                return;
            }
        }
        let visible_len = self.visible_sidebar_items_ref().len();
        if visible_len == 0 {
            self.table_state.select(None);
            self.sidebar.selected_sidebar_key = None;
            return;
        }

        let current = self.table_state.selected();
        let Some(i) = ({
            let items = self.visible_sidebar_items_ref();
            Self::next_navigable_sidebar_index(items, current, true)
        }) else {
            return;
        };
        let selected_key = self
            .visible_sidebar_items_ref()
            .get(i)
            .map(|item| item.key().to_string());
        log_debug!("nav: next (panel) index={}", i);
        self.table_state.select(Some(i));
        self.sidebar.selected_sidebar_key = selected_key;
        self.clear_unread_stop_for_selected_panel();
        self.debounce_preview_after_navigation();
        self.update_tree_for_selection();
        self.dirty = true;
    }

    pub fn previous(&mut self) {
        if self.sidebar.show_tree {
            if let Some(ref mut tree) = self.sidebar.file_tree {
                log_debug!("nav: previous (tree) selected={:?}", tree.state.selected());
                tree.previous();
                self.dirty = true;
                return;
            }
        }
        let visible_len = self.visible_sidebar_items_ref().len();
        if visible_len == 0 {
            self.table_state.select(None);
            self.sidebar.selected_sidebar_key = None;
            return;
        }

        let current = self.table_state.selected();
        let Some(i) = ({
            let items = self.visible_sidebar_items_ref();
            Self::next_navigable_sidebar_index(items, current, false)
        }) else {
            return;
        };
        let selected_key = self
            .visible_sidebar_items_ref()
            .get(i)
            .map(|item| item.key().to_string());
        log_debug!("nav: previous (panel) index={}", i);
        self.table_state.select(Some(i));
        self.sidebar.selected_sidebar_key = selected_key;
        self.clear_unread_stop_for_selected_panel();
        self.debounce_preview_after_navigation();
        self.update_tree_for_selection();
        self.dirty = true;
    }

    pub fn move_selected_sidebar_item_up(&mut self) -> bool {
        self.move_selected_sidebar_item_by(-1)
    }

    pub fn move_selected_sidebar_item_down(&mut self) -> bool {
        self.move_selected_sidebar_item_by(1)
    }

    fn move_selected_sidebar_item_by(&mut self, delta: isize) -> bool {
        if self.sidebar.show_tree
            || self.thread_list_view() != crate::app::state::sidebar::ThreadListView::Normal
        {
            return false;
        }

        self.ensure_visible_sidebar_items_cache();
        let selected_key = self.sidebar.selected_sidebar_key.clone();
        let selected_index = self.table_state.selected();
        let folders = self.sidebar.sidebar_folders_cache.clone();
        let items = self.sidebar.visible_sidebar_items_cache.clone();
        let Some(selected_index) = selected_key
            .as_deref()
            .and_then(|key| items.iter().position(|item| item.key() == key))
            .or(selected_index)
        else {
            return false;
        };

        let mut entries = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                if !Self::sidebar_item_is_navigable(&items, index, item) {
                    return None;
                }
                item_thread(item, &folders).map(|thread| (index, item.key().to_string(), thread))
            })
            .collect::<Vec<_>>();
        let Some(current) = entries.iter().position(|entry| entry.0 == selected_index) else {
            return false;
        };
        let Some(target) = (if delta < 0 {
            current.checked_sub(1)
        } else {
            (current + 1 < entries.len()).then_some(current + 1)
        }) else {
            return false;
        };

        entries.swap(current, target);
        let base = crate::app::unix_now_ts().saturating_mul(1000);
        let total = entries.len() as i64;
        for (order, (_, _, thread)) in entries.iter().enumerate() {
            let rank = base + total - order as i64;
            for key in thread.sort_activity_keys() {
                self.sidebar.thread_sort_activity.insert(key, rank);
            }
        }

        let moved_key = entries[target].1.clone();
        self.sidebar.selected_sidebar_key = Some(moved_key);
        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        self.dirty = true;
        true
    }

    pub fn jump_to_sidebar_index(&mut self, index: usize) -> bool {
        self.select_sidebar_index(index, true)
    }

    pub fn jump_to(&mut self, index: usize) {
        let target_sidebar_index = {
            let items = self.visible_sidebar_items_ref();
            Self::nth_visible_thread_sidebar_index(items, index)
        };
        let Some(target_sidebar_index) = target_sidebar_index else {
            return;
        };
        self.select_sidebar_index(target_sidebar_index, true);
    }
}

fn item_thread(
    item: &SidebarItem,
    folders: &[crate::sidebar::SidebarFolder],
) -> Option<SidebarThread> {
    match item {
        SidebarItem::Thread(thread) => Some(thread.as_ref().clone()),
        SidebarItem::Folder(folder) => folders
            .iter()
            .find(|candidate| candidate.key == folder.key)
            .and_then(crate::sidebar::SidebarFolder::primary_thread),
    }
}
