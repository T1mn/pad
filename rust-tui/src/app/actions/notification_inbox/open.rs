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
