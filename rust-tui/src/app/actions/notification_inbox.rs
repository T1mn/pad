mod mutate {
    use super::super::*;
    use crate::notification_inbox::NotificationEntry;

    impl App {
        pub fn mark_selected_notification_read(&mut self) -> bool {
            let Some(id) = self.selected_notification_id().map(str::to_string) else {
                return false;
            };
            let changed = self.notification_inbox.mark_read(&id);
            if changed {
                super::persist::persist_mark_read(&id);
                self.dirty = true;
            }
            changed
        }

        pub fn mark_all_notifications_read(&mut self) -> usize {
            let changed = self.notification_inbox.mark_all_read();
            if changed > 0 {
                super::persist::persist_mark_all_read();
                self.dirty = true;
            }
            changed
        }

        pub fn delete_selected_notification(&mut self) -> bool {
            let Some(id) = self.selected_notification_id().map(str::to_string) else {
                return false;
            };
            let changed = self.notification_inbox.delete(&id);
            if changed {
                super::persist::persist_delete(&id);
                let len = self.notification_inbox.entries.len();
                self.notification_inbox_selected =
                    self.notification_inbox_selected.min(len.saturating_sub(1));
                self.dirty = true;
            }
            changed
        }

        pub fn push_notification_entry(&mut self, entry: NotificationEntry) {
            self.notification_inbox.push(entry.clone());
            super::persist::persist_append(entry);
            self.dirty = true;
        }
    }
}
mod open {
    use super::super::*;

    impl App {
        pub fn open_notification_inbox(&mut self) {
            self.notification_inbox = crate::notification_inbox::load();
            self.notification_inbox_selected = selection_after_reload(
                self.notification_inbox_selected,
                self.notification_inbox.entries.len(),
            );
            self.mode = Mode::NotificationInbox;
            self.dirty = true;
        }

        pub fn close_notification_inbox(&mut self) {
            self.mode = Mode::Normal;
            self.dirty = true;
        }
    }

    fn selection_after_reload(current: usize, len: usize) -> usize {
        if len == 0 {
            0
        } else {
            current.min(len.saturating_sub(1))
        }
    }
}
mod persist {
    use crate::log_debug;
    use crate::notification_inbox::NotificationEntry;

    pub(super) fn persist_mark_read(id: &str) {
        if should_persist_inbox_from_app() {
            if let Err(err) = crate::notification_inbox::mark_read(id) {
                log_debug!("notification_inbox: mark_read failed: {}", err);
            }
        }
    }

    pub(super) fn persist_mark_all_read() {
        if should_persist_inbox_from_app() {
            if let Err(err) = crate::notification_inbox::mark_all_read() {
                log_debug!("notification_inbox: mark_all_read failed: {}", err);
            }
        }
    }

    pub(super) fn persist_delete(id: &str) {
        if should_persist_inbox_from_app() {
            if let Err(err) = crate::notification_inbox::delete(id) {
                log_debug!("notification_inbox: delete failed: {}", err);
            }
        }
    }

    pub(super) fn persist_append(entry: NotificationEntry) {
        if should_persist_inbox_from_app() {
            if let Err(err) = crate::notification_inbox::append(entry) {
                log_debug!("notification_inbox: append failed: {}", err);
            }
        }
    }

    #[cfg(not(test))]
    fn should_persist_inbox_from_app() -> bool {
        true
    }

    #[cfg(test)]
    fn should_persist_inbox_from_app() -> bool {
        std::env::var_os("PAD_TEST_PERSIST_INBOX").is_some()
    }
}
mod selection {
    use super::super::*;

    impl App {
        pub fn move_notification_selection(&mut self, delta: isize) {
            self.notification_inbox_selected = next_selection(
                self.notification_inbox_selected,
                self.notification_inbox.entries.len(),
                delta,
            );
            self.dirty = true;
        }

        pub fn selected_notification_id(&self) -> Option<&str> {
            self.notification_inbox
                .entries
                .get(self.notification_inbox_selected)
                .map(|entry| entry.id.as_str())
        }
    }

    fn next_selection(current: usize, count: usize, delta: isize) -> usize {
        if count == 0 {
            return 0;
        }
        let max = count.saturating_sub(1);
        let current = current.min(max);
        if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current.saturating_add(delta as usize).min(max)
        }
    }
}

#[cfg(test)]
#[path = "notification_inbox_tests.rs"]
mod tests;
