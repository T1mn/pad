use super::super::App;
use crate::log_debug;

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
        self.invalidate_preview();
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
        self.invalidate_preview();
        self.update_tree_for_selection();
        self.dirty = true;
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
