use super::super::super::App;

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
}
