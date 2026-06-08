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
