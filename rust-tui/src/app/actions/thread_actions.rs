mod confirm {
    use crate::app::actions::helpers::{
        failure_toast_title, success_toast_title, thread_action_subject,
    };
    use crate::app::{App, Mode, PendingThreadAction, ThreadActionKind};
    use crate::model::AgentType;
    use crate::sidebar::SidebarThread;

    impl App {
        pub fn open_thread_action_confirm(
            &mut self,
            thread: SidebarThread,
            kind: ThreadActionKind,
        ) {
            self.sidebar.pending_thread_action = Some(PendingThreadAction { thread, kind });
            self.sidebar.thread_meta_editing = false;
            self.sidebar.thread_meta_target = None;
            self.sidebar.thread_meta_buffer.clear();
            self.mode = Mode::ThreadActionConfirm;
            self.dirty = true;
        }

        pub fn close_thread_action_confirm(&mut self) {
            self.sidebar.pending_thread_action = None;
            self.sidebar.thread_meta_editing = false;
            self.sidebar.thread_meta_target = None;
            self.sidebar.thread_meta_buffer.clear();
            self.mode = Mode::Normal;
            self.dirty = true;
        }

        pub fn confirm_thread_action(&mut self) -> bool {
            let Some(action) = self.sidebar.pending_thread_action.clone() else {
                self.mode = Mode::Normal;
                self.dirty = true;
                return false;
            };
            self.sidebar.pending_thread_action = None;
            self.mode = Mode::Normal;

            let Some(session_id) = action.thread.session_id.as_deref() else {
                self.dirty = true;
                return false;
            };

            let result = execute_thread_action(&action, session_id);
            let ok = result.is_ok();

            match &result {
                Ok(()) => self.show_action_toast(
                    success_toast_title(self.locale, action.kind, action.thread.agent_type.clone()),
                    &thread_action_subject(&action.thread),
                ),
                Err(err) => self.show_action_toast(
                    failure_toast_title(self.locale, action.kind, action.thread.agent_type.clone()),
                    &err.to_string(),
                ),
            }

            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
            self.invalidate_preview();
            self.focus_panel();
            self.dirty = true;
            ok
        }
    }

    fn execute_thread_action(
        action: &PendingThreadAction,
        session_id: &str,
    ) -> std::io::Result<()> {
        match action.kind {
            ThreadActionKind::Archive => archive_thread(&action.thread.agent_type, session_id),
            ThreadActionKind::Unarchive => unarchive_thread(&action.thread.agent_type, session_id),
            ThreadActionKind::Restore => crate::thread_meta::set_thread_deleted(
                &action.thread.agent_type.to_string(),
                session_id,
                false,
            ),
        }
    }

    fn archive_thread(agent_type: &AgentType, session_id: &str) -> std::io::Result<()> {
        match agent_type {
            AgentType::Codex => crate::codex_state::archive_thread(session_id),
            AgentType::Claude => crate::claude_history::archive_thread(session_id),
            AgentType::Gemini => crate::gemini_history::archive_thread(session_id),
            AgentType::OpenCode => crate::opencode_history::archive_thread(session_id),
            _ => unsupported_thread_action("archive"),
        }
    }

    fn unarchive_thread(agent_type: &AgentType, session_id: &str) -> std::io::Result<()> {
        match agent_type {
            AgentType::Codex => crate::codex_state::unarchive_thread(session_id),
            AgentType::Claude => crate::claude_history::unarchive_thread(session_id),
            AgentType::Gemini => crate::gemini_history::unarchive_thread(session_id),
            AgentType::OpenCode => crate::opencode_history::unarchive_thread(session_id),
            _ => unsupported_thread_action("restore"),
        }
    }

    fn unsupported_thread_action(action: &str) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{action} is not supported for this agent type"),
        ))
    }
}
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
