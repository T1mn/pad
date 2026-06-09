use super::*;
use crate::notification_inbox::model::NotificationEntry;

#[test]
fn save_and_load_round_trips_entries() {
    let dir = crate::test_support::temp_path("pad-inbox-test", "round-trip");
    let path = dir.join("inbox.json");
    let mut inbox = NotificationInbox::empty();
    inbox.entries = vec![NotificationEntry {
        id: "one".into(),
        ts: 10,
        title: "done".into(),
        body: "body".into(),
        ..NotificationEntry::default()
    }];

    save_to_path(&path, &inbox).unwrap();
    let loaded = load_from_path(&path);

    assert_eq!(loaded.entries.len(), 1);
    assert_eq!(loaded.entries[0].id, "one");
    let _ = std::fs::remove_dir_all(dir);
}
