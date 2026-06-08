use super::NotificationEntry;
use serde::{Deserialize, Serialize};

pub const INBOX_VERSION: u32 = 1;
const MAX_INBOX_ENTRIES: usize = 500;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NotificationInbox {
    pub version: u32,
    pub entries: Vec<NotificationEntry>,
}

impl NotificationInbox {
    pub fn normalized(mut self) -> Self {
        if self.version == 0 {
            self.version = INBOX_VERSION;
        }
        sort_entries_newest_first(&mut self.entries);
        self.entries.truncate(MAX_INBOX_ENTRIES);
        self
    }

    pub fn unread_count(&self) -> usize {
        self.entries.iter().filter(|entry| !entry.read).count()
    }

    pub fn push(&mut self, entry: NotificationEntry) {
        if self.version == 0 {
            self.version = INBOX_VERSION;
        }
        self.entries.insert(0, entry);
        sort_entries_newest_first(&mut self.entries);
        self.entries.truncate(MAX_INBOX_ENTRIES);
    }

    pub fn mark_read(&mut self, id: &str) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) else {
            return false;
        };
        let changed = !entry.read;
        entry.read = true;
        changed
    }

    pub fn mark_all_read(&mut self) -> usize {
        let mut changed = 0;
        for entry in &mut self.entries {
            if !entry.read {
                entry.read = true;
                changed += 1;
            }
        }
        changed
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != before
    }
}

fn sort_entries_newest_first(entries: &mut [NotificationEntry]) {
    entries.sort_by(|left, right| right.ts.cmp(&left.ts).then_with(|| right.id.cmp(&left.id)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, ts: i64, read: bool) -> NotificationEntry {
        NotificationEntry {
            id: id.into(),
            ts,
            read,
            title: id.into(),
            ..NotificationEntry::default()
        }
    }

    #[test]
    fn inbox_keeps_newest_first_and_counts_unread() {
        let mut inbox = NotificationInbox::default();
        inbox.push(entry("old", 1, false));
        inbox.push(entry("new", 2, true));

        assert_eq!(inbox.entries[0].id, "new");
        assert_eq!(inbox.unread_count(), 1);
    }

    #[test]
    fn mark_read_and_delete_report_changes() {
        let mut inbox = NotificationInbox::default();
        inbox.push(entry("a", 1, false));

        assert!(inbox.mark_read("a"));
        assert!(!inbox.mark_read("a"));
        assert!(inbox.delete("a"));
        assert!(!inbox.delete("a"));
    }
}
