use super::super::super::App;
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
        self.move_sidebar_selection(true);
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
        self.move_sidebar_selection(false);
    }

    fn move_sidebar_selection(&mut self, forward: bool) {
        let visible_len = self.visible_sidebar_items_ref().len();
        if visible_len == 0 {
            self.table_state.select(None);
            self.sidebar.selected_sidebar_key = None;
            return;
        }

        let current = self.table_state.selected();
        let Some(index) = ({
            let items = self.visible_sidebar_items_ref();
            Self::next_navigable_sidebar_index(items, current, forward)
        }) else {
            return;
        };
        let selected_key = self
            .visible_sidebar_items_ref()
            .get(index)
            .map(|item| item.key().to_string());
        let direction = if forward { "next" } else { "previous" };
        log_debug!("nav: {} (panel) index={}", direction, index);
        self.table_state.select(Some(index));
        self.sidebar.selected_sidebar_key = selected_key;
        self.clear_unread_stop_for_selected_panel();
        self.debounce_preview_after_navigation();
        self.update_tree_for_selection();
        self.dirty = true;
    }
}
