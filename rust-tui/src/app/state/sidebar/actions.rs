use crate::sidebar::SidebarThread;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadActionKind {
    Archive,
    Unarchive,
    Restore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadListView {
    Normal,
    Archived,
    Trash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadMetaEditKind {
    Title,
    Tags,
}

#[derive(Clone)]
pub struct PendingThreadAction {
    pub thread: SidebarThread,
    pub kind: ThreadActionKind,
}
