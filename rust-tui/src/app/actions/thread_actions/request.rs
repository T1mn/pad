use super::target::selected_thread_action_target;
use crate::app::state::ThreadListView;
use crate::app::{App, ThreadActionKind};

impl App {
    pub fn request_archive_selected_thread(&mut self) -> bool {
        if self.thread_list_view() == ThreadListView::Trash {
            return false;
        }
        let Some(thread) = selected_thread_action_target(self, false) else {
            return false;
        };
        self.open_thread_action_confirm(thread, ThreadActionKind::Archive);
        true
    }

    pub fn request_unarchive_selected_thread(&mut self) -> bool {
        let target_archived = self.thread_list_view() == ThreadListView::Archived;
        let Some(thread) = selected_thread_action_target(self, target_archived) else {
            return false;
        };
        let kind = if self.thread_list_view() == ThreadListView::Trash {
            ThreadActionKind::Restore
        } else {
            ThreadActionKind::Unarchive
        };
        self.open_thread_action_confirm(thread, kind);
        true
    }
}
