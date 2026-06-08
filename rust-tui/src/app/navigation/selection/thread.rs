use super::super::super::App;
use crate::app::state::sidebar::ThreadListView;
use crate::model::AgentPanel;
use crate::sidebar::{SidebarFolder, SidebarItem, SidebarThread};

impl App {
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

    pub fn preview_target_thread(&mut self) -> Option<SidebarThread> {
        let target_key = self.preview.pane_id.clone()?;
        let mut thread = self
            .sidebar_folders_ref()
            .iter()
            .flat_map(|folder| folder.threads.iter())
            .find(|thread| thread.key == target_key)
            .map(|thread| thread.as_ref().clone())?;
        self.apply_cached_preview_to_thread(&mut thread);
        Some(thread)
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

    pub fn selected_panel(&mut self) -> Option<&AgentPanel> {
        let pane_id = self
            .selected_preview_thread()
            .and_then(|thread| thread.live_pane_id)?;
        self.panels.iter().find(|panel| panel.pane_id == pane_id)
    }
}
