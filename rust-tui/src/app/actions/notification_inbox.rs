mod mutate;
mod open;
mod persist;
mod selection;

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::notification_inbox::NotificationEntry;

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
