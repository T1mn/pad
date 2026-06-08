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
