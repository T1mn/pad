use serde::{Deserialize, Serialize};

pub const INBOX_VERSION: u32 = 1;
pub const MAX_INBOX_ENTRIES: usize = 500;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NotificationInbox {
    pub version: u32,
    pub entries: Vec<NotificationEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationEntry {
    pub id: String,
    pub ts: i64,
    pub event: String,
    pub agent_type: String,
    pub title: String,
    pub body: String,
    pub working_dir: Option<String>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub pane_id: Option<String>,
    pub read: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationDraft {
    pub event: String,
    pub agent_type: String,
    pub title: String,
    pub body: String,
    pub working_dir: Option<String>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub pane_id: Option<String>,
}

impl NotificationInbox {
    pub fn normalized(mut self) -> Self {
        if self.version == 0 {
            self.version = INBOX_VERSION;
        }
        self.entries
            .sort_by(|left, right| right.ts.cmp(&left.ts).then_with(|| right.id.cmp(&left.id)));
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
        self.entries
            .sort_by(|left, right| right.ts.cmp(&left.ts).then_with(|| right.id.cmp(&left.id)));
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

impl Default for NotificationEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            ts: 0,
            event: String::new(),
            agent_type: String::new(),
            title: String::new(),
            body: String::new(),
            working_dir: None,
            session_id: None,
            transcript_path: None,
            pane_id: None,
            read: false,
        }
    }
}

impl NotificationEntry {
    pub fn from_draft(draft: NotificationDraft, ts: i64) -> Self {
        Self {
            id: new_entry_id(ts),
            ts,
            event: draft.event,
            agent_type: draft.agent_type,
            title: draft.title,
            body: draft.body,
            working_dir: draft.working_dir,
            session_id: draft.session_id,
            transcript_path: draft.transcript_path,
            pane_id: draft.pane_id,
            read: false,
        }
    }
}

pub fn new_entry_id(ts: i64) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("n-{ts}-{nanos}-{}", std::process::id())
}

pub fn short_time(ts: i64) -> String {
    if ts <= 0 {
        return "unknown".to_string();
    }
    let now = crate::app::unix_now_ts();
    let age = now.saturating_sub(ts);
    if age < 60 {
        format!("{age}s ago")
    } else if age < 3600 {
        format!("{}m ago", age / 60)
    } else if age < 86_400 {
        format!("{}h ago", age / 3600)
    } else {
        format!("{}d ago", age / 86_400)
    }
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
