mod model;
mod storage;

pub use model::{short_time, NotificationDraft, NotificationEntry, NotificationInbox};
pub use storage::{append, delete, load, mark_all_read, mark_read};
