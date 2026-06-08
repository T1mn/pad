use crate::app::state::ThreadListView;
use crate::app::{App, Mode};

impl App {
    pub fn toggle_archived_threads_view(&mut self) {
        self.sidebar.thread_list_view = if self.thread_list_view() != ThreadListView::Normal {
            ThreadListView::Normal
        } else {
            ThreadListView::Archived
        };
        self.reset_thread_list_view_state();
    }

    pub fn open_trash_threads_view(&mut self) {
        self.sidebar.thread_list_view = ThreadListView::Trash;
        self.reset_thread_list_view_state();
    }

    fn reset_thread_list_view_state(&mut self) {
        self.sidebar.pending_thread_action = None;
        self.sidebar.pending_sidebar_selection_index = None;
        self.settings_open = false;
        self.mode = Mode::Normal;
        self.sidebar.selected_sidebar_key = None;
        self.table_state.select(None);
        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        self.invalidate_preview();
        self.focus_panel();
        self.dirty = true;
    }
}
