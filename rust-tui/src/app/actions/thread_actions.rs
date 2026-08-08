mod confirm;
mod request {
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
}
mod target {
    use crate::app::state::ThreadListView;
    use crate::app::App;
    use crate::model::AgentType;
    use crate::sidebar::{SidebarItem, SidebarThread};

    pub(super) fn selected_thread_action_target(
        app: &mut App,
        archived: bool,
    ) -> Option<SidebarThread> {
        match app.selected_sidebar_item()? {
            SidebarItem::Thread(thread)
                if matches!(
                    thread.agent_type,
                    AgentType::Codex | AgentType::Claude | AgentType::Gemini | AgentType::OpenCode
                ) && (app.thread_list_view() == ThreadListView::Trash
                    || thread.archived == archived)
                    && (app.thread_list_view() != ThreadListView::Trash || thread.deleted)
                    && thread.session_id.is_some() =>
            {
                Some(thread.as_ref().clone())
            }
            _ => None,
        }
    }
}
mod view {
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
}

#[cfg(test)]
#[path = "thread_actions_tests.rs"]
mod thread_actions_tests;
