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
