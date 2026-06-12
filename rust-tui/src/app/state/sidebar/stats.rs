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
