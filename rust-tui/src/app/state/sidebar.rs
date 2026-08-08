mod actions {
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
}
mod space {
    use std::time::Instant;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) enum PendingSidebarSpaceActionKind {
        ToggleFolder(String),
        CollapseParentFolder(String),
    }

    #[derive(Clone, Debug)]
    pub(crate) struct PendingSidebarSpaceAction {
        pub kind: PendingSidebarSpaceActionKind,
        pub deadline: Instant,
    }
}
mod state;
mod stats {
    use super::actions::ThreadListView;
    use crate::sidebar::SidebarItem;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct VisibleSidebarStats {
        pub item_count: usize,
        pub thread_count: usize,
        pub row_count: usize,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct PreferredPanelWidthCache {
        pub width: u16,
        pub locale: crate::i18n::Locale,
        pub thread_list_view: ThreadListView,
        pub live_only: bool,
        pub manual_width: Option<u16>,
    }

    impl VisibleSidebarStats {
        pub fn from_items(items: &[SidebarItem]) -> Self {
            let mut stats = Self {
                item_count: items.len(),
                thread_count: 0,
                row_count: 0,
            };
            for item in items {
                match item {
                    SidebarItem::Folder(_) => stats.row_count += 1,
                    SidebarItem::Thread(_) => {
                        stats.thread_count += 1;
                        stats.row_count += 1;
                    }
                }
            }
            stats
        }
    }
}

pub use actions::{PendingThreadAction, ThreadActionKind, ThreadListView, ThreadMetaEditKind};
pub(crate) use space::{PendingSidebarSpaceAction, PendingSidebarSpaceActionKind};
pub use state::SidebarState;
pub use stats::{PreferredPanelWidthCache, VisibleSidebarStats};
