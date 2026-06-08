use super::helpers::{parse_thread_tags, thread_meta_save_failed_title, thread_meta_toast};
use super::*;

impl App {
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
}
