use super::helpers::{
    archive_deleted_thread, delete_failed_title, delete_hide_failed_title, failure_toast_title,
    parse_thread_tags, success_toast_title, thread_action_subject, thread_meta_save_failed_title,
    thread_meta_toast,
};
use super::*;

impl App {
    pub fn delete_panel(&mut self, panel: &crate::model::AgentPanel) {
        self.sidebar.pending_sidebar_selection_index = self.table_state.selected();
        let target_thread = self
            .selected_preview_thread()
            .filter(|thread| thread.live_pane_id.as_deref() == Some(panel.pane_id.as_str()));

        log_debug!(
            "delete_panel: pane_id={} target_thread={} session_id={:?} agent_type={:?}",
            panel.pane_id,
            target_thread
                .as_ref()
                .map(|thread| thread.key.as_str())
                .unwrap_or("-"),
            target_thread
                .as_ref()
                .and_then(|thread| thread.session_id.as_deref()),
            target_thread.as_ref().map(|thread| &thread.agent_type),
        );

        let kill_result = std::process::Command::new("tmux")
            .args(["kill-pane", "-t", &panel.pane_id])
            .output();

        match kill_result {
            Ok(output) if output.status.success() => {
                self.apply_deleted_panel_locally(&panel.pane_id);
                if let Some(thread) = target_thread.as_ref() {
                    if let Err(err) = archive_deleted_thread(thread) {
                        self.show_action_toast(
                            delete_hide_failed_title(self.locale),
                            &err.to_string(),
                        );
                    }
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let message = if stderr.is_empty() {
                    format!("tmux kill-pane exited with {}", output.status)
                } else {
                    stderr
                };
                self.show_action_toast(delete_failed_title(self.locale), &message);
            }
            Err(err) => {
                self.show_action_toast(delete_failed_title(self.locale), &err.to_string());
            }
        }

        self.refresh_panels();
    }

    pub(crate) fn apply_deleted_panel_locally(&mut self, pane_id: &str) {
        let original_len = self.panels.len();
        self.panels.retain(|panel| panel.pane_id != pane_id);
        if self.panels.len() == original_len {
            return;
        }

        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        if self.selected_panel().is_none() {
            self.focus_panel();
        }
        self.invalidate_preview();
        self.dirty = true;
    }

    pub fn refresh_panels(&mut self) {
        if !self.scan_in_progress {
            self.trigger_async_scan();
        }
    }

    pub fn toggle_archived_threads_view(&mut self) {
        self.sidebar.archived_threads_view = !self.sidebar.archived_threads_view;
        self.sidebar.pending_thread_action = None;
        self.sidebar.pending_sidebar_selection_index = None;
        self.mode = Mode::Normal;
        self.sidebar.selected_sidebar_key = None;
        self.table_state.select(None);
        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        self.invalidate_preview();
        self.focus_panel();
        self.dirty = true;
    }

    pub fn request_archive_selected_thread(&mut self) -> bool {
        let Some(thread) = self.selected_thread_action_target(false) else {
            return false;
        };
        self.open_thread_action_confirm(thread, ThreadActionKind::Archive);
        true
    }

    pub fn request_unarchive_selected_thread(&mut self) -> bool {
        let Some(thread) = self.selected_thread_action_target(true) else {
            return false;
        };
        self.open_thread_action_confirm(thread, ThreadActionKind::Unarchive);
        true
    }

    pub fn open_thread_action_confirm(&mut self, thread: SidebarThread, kind: ThreadActionKind) {
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

        let result = match action.kind {
            ThreadActionKind::Archive => match action.thread.agent_type {
                AgentType::Codex => crate::codex_state::archive_thread(session_id),
                AgentType::Claude => crate::claude_history::archive_thread(session_id),
                AgentType::Gemini => crate::gemini_history::archive_thread(session_id),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "archive is not supported for this agent type",
                )),
            },
            ThreadActionKind::Unarchive => match action.thread.agent_type {
                AgentType::Codex => crate::codex_state::unarchive_thread(session_id),
                AgentType::Claude => crate::claude_history::unarchive_thread(session_id),
                AgentType::Gemini => crate::gemini_history::unarchive_thread(session_id),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "restore is not supported for this agent type",
                )),
            },
        };
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

    pub fn open_thread_title_editor(&mut self) -> bool {
        let Some(SidebarItem::Thread(thread)) = self.selected_sidebar_item() else {
            return false;
        };

        self.sidebar.pending_thread_action = None;
        self.sidebar.thread_meta_editing = true;
        self.sidebar.thread_meta_edit_kind = ThreadMetaEditKind::Title;
        self.sidebar.thread_meta_buffer = thread.title.clone();
        self.sidebar.thread_meta_target = Some(thread.as_ref().clone());
        self.mode = Mode::ThreadActionConfirm;
        self.dirty = true;
        true
    }

    pub fn open_thread_tags_editor(&mut self) -> bool {
        let Some(SidebarItem::Thread(thread)) = self.selected_sidebar_item() else {
            return false;
        };

        self.sidebar.pending_thread_action = None;
        self.sidebar.thread_meta_editing = true;
        self.sidebar.thread_meta_edit_kind = ThreadMetaEditKind::Tags;
        self.sidebar.thread_meta_buffer = thread.tags.join(", ");
        self.sidebar.thread_meta_target = Some(thread.as_ref().clone());
        self.mode = Mode::ThreadActionConfirm;
        self.dirty = true;
        true
    }

    pub fn cancel_thread_meta_edit(&mut self) {
        self.sidebar.thread_meta_editing = false;
        self.sidebar.thread_meta_target = None;
        self.sidebar.thread_meta_buffer.clear();
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    pub fn commit_thread_meta_edit(&mut self) -> bool {
        let Some(thread) = self.sidebar.thread_meta_target.clone() else {
            self.cancel_thread_meta_edit();
            return false;
        };

        let input = self.sidebar.thread_meta_buffer.trim().to_string();
        let result = match self.sidebar.thread_meta_edit_kind {
            ThreadMetaEditKind::Title => self.persist_thread_title_override(&thread, &input),
            ThreadMetaEditKind::Tags => {
                let tags = parse_thread_tags(&input);
                self.persist_thread_tags(&thread, &tags)
            }
        };

        let ok = result.is_ok();
        if ok {
            let (title, content) =
                thread_meta_toast(self.locale, self.sidebar.thread_meta_edit_kind, &input);
            self.show_action_toast(title, &content);
        } else if let Err(err) = result {
            self.show_action_toast(thread_meta_save_failed_title(self.locale), &err.to_string());
        }

        if ok {
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
            self.invalidate_preview();
        }
        self.cancel_thread_meta_edit();
        ok
    }

    fn persist_thread_title_override(
        &mut self,
        thread: &SidebarThread,
        title: &str,
    ) -> std::io::Result<()> {
        let session_id = thread.session_id.as_deref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "selected thread is missing session id",
            )
        })?;
        log_debug!(
            "thread_meta: title override save requested thread={} title={}",
            thread.key,
            title
        );
        crate::thread_meta::upsert_thread_meta(
            &thread.agent_type.to_string(),
            session_id,
            Some(title),
            thread.note.as_deref(),
            thread.pinned,
        )
    }

    fn persist_thread_tags(
        &mut self,
        thread: &SidebarThread,
        tags: &[String],
    ) -> std::io::Result<()> {
        let session_id = thread.session_id.as_deref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "selected thread is missing session id",
            )
        })?;
        log_debug!(
            "thread_meta: tags save requested thread={} tags={:?}",
            thread.key,
            tags
        );
        crate::thread_meta::replace_thread_tags(&thread.agent_type.to_string(), session_id, tags)
    }

    fn selected_thread_action_target(&mut self, archived: bool) -> Option<SidebarThread> {
        match self.selected_sidebar_item()? {
            SidebarItem::Thread(thread)
                if matches!(
                    thread.agent_type,
                    AgentType::Codex | AgentType::Claude | AgentType::Gemini
                ) && thread.archived == archived
                    && thread.session_id.is_some() =>
            {
                Some(thread.as_ref().clone())
            }
            _ => None,
        }
    }
}
