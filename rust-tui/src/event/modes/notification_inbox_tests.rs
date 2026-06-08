use super::handle_notification_inbox_mode;
use crate::app::state::Mode;
use crate::app::App;
use crate::notification_inbox::NotificationEntry;
use crossterm::event::KeyCode;

#[test]
fn escape_closes_inbox() {
    let mut app = App::new();
    app.mode = Mode::NotificationInbox;
    app.notification_inbox
        .entries
        .push(NotificationEntry::default());

    handle_notification_inbox_mode(&mut app, KeyCode::Esc);

    assert!(matches!(app.mode, Mode::Normal));
}
