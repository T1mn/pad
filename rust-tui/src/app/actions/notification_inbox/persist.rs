use crate::log_debug;
use crate::notification_inbox::NotificationEntry;

pub(super) fn persist_mark_read(id: &str) {
    if should_persist_inbox_from_app() {
        if let Err(err) = crate::notification_inbox::mark_read(id) {
            log_debug!("notification_inbox: mark_read failed: {}", err);
        }
    }
}

pub(super) fn persist_mark_all_read() {
    if should_persist_inbox_from_app() {
        if let Err(err) = crate::notification_inbox::mark_all_read() {
            log_debug!("notification_inbox: mark_all_read failed: {}", err);
        }
    }
}

pub(super) fn persist_delete(id: &str) {
    if should_persist_inbox_from_app() {
        if let Err(err) = crate::notification_inbox::delete(id) {
            log_debug!("notification_inbox: delete failed: {}", err);
        }
    }
}

pub(super) fn persist_append(entry: NotificationEntry) {
    if should_persist_inbox_from_app() {
        if let Err(err) = crate::notification_inbox::append(entry) {
            log_debug!("notification_inbox: append failed: {}", err);
        }
    }
}

#[cfg(not(test))]
fn should_persist_inbox_from_app() -> bool {
    true
}

#[cfg(test)]
fn should_persist_inbox_from_app() -> bool {
    std::env::var_os("PAD_TEST_PERSIST_INBOX").is_some()
}
