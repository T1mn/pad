use super::*;
use crate::notification_inbox::NotificationEntry;

impl App {
    pub fn open_notification_inbox(&mut self) {
        self.notification_inbox = crate::notification_inbox::load();
        if self.notification_inbox.entries.is_empty() {
            self.notification_inbox_selected = 0;
        } else {
            self.notification_inbox_selected = self
                .notification_inbox_selected
                .min(self.notification_inbox.entries.len().saturating_sub(1));
        }
        self.mode = Mode::NotificationInbox;
        self.dirty = true;
    }

    pub fn close_notification_inbox(&mut self) {
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    pub fn move_notification_selection(&mut self, delta: isize) {
        let count = self.notification_inbox.entries.len();
        if count == 0 {
            self.notification_inbox_selected = 0;
            self.dirty = true;
            return;
        }
        let max = count.saturating_sub(1);
        let current = self.notification_inbox_selected.min(max);
        self.notification_inbox_selected = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current.saturating_add(delta as usize).min(max)
        };
        self.dirty = true;
    }

    pub fn selected_notification_id(&self) -> Option<&str> {
        self.notification_inbox
            .entries
            .get(self.notification_inbox_selected)
            .map(|entry| entry.id.as_str())
    }

    pub fn mark_selected_notification_read(&mut self) -> bool {
        let Some(id) = self.selected_notification_id().map(str::to_string) else {
            return false;
        };
        let changed = self.notification_inbox.mark_read(&id);
        if changed {
            if should_persist_inbox_from_app() {
                if let Err(err) = crate::notification_inbox::mark_read(&id) {
                    log_debug!("notification_inbox: mark_read failed: {}", err);
                }
            }
            self.dirty = true;
        }
        changed
    }

    pub fn mark_all_notifications_read(&mut self) -> usize {
        let changed = self.notification_inbox.mark_all_read();
        if changed > 0 {
            if should_persist_inbox_from_app() {
                if let Err(err) = crate::notification_inbox::mark_all_read() {
                    log_debug!("notification_inbox: mark_all_read failed: {}", err);
                }
            }
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
            if should_persist_inbox_from_app() {
                if let Err(err) = crate::notification_inbox::delete(&id) {
                    log_debug!("notification_inbox: delete failed: {}", err);
                }
            }
            let len = self.notification_inbox.entries.len();
            self.notification_inbox_selected =
                self.notification_inbox_selected.min(len.saturating_sub(1));
            self.dirty = true;
        }
        changed
    }

    pub fn push_notification_entry(&mut self, entry: NotificationEntry) {
        self.notification_inbox.push(entry.clone());
        if should_persist_inbox_from_app() {
            if let Err(err) = crate::notification_inbox::append(entry) {
                log_debug!("notification_inbox: append failed: {}", err);
            }
        }
        self.dirty = true;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_selection_clamps_to_available_entries() {
        let mut app = App::new();
        app.notification_inbox.entries = vec![
            NotificationEntry {
                id: "a".into(),
                ..NotificationEntry::default()
            },
            NotificationEntry {
                id: "b".into(),
                ..NotificationEntry::default()
            },
        ];
        app.move_notification_selection(99);
        assert_eq!(app.notification_inbox_selected, 1);
        app.move_notification_selection(-99);
        assert_eq!(app.notification_inbox_selected, 0);
    }
}
