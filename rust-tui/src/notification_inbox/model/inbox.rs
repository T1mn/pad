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
    pub fn empty() -> Self {
        Self {
            version: INBOX_VERSION,
            entries: Vec::new(),
        }
    }

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
#[path = "inbox_tests.rs"]
mod tests;
