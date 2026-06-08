mod draft;
mod emit;
mod request;

use crate::notification_inbox::NotificationDraft;
use crate::notify::NotificationRequest;

pub(super) use draft::{completion_notification_for_activity, completion_notification_for_panel};
pub(super) use emit::emit_completion_notification;
#[cfg(test)]
pub(super) use request::build_completion_notification;

#[derive(Clone, Debug)]
pub(super) struct PendingNotification {
    pub(super) request: NotificationRequest,
    pub(super) draft: NotificationDraft,
}
