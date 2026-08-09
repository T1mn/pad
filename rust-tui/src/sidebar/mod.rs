pub(crate) mod build;
pub(crate) mod display;
mod model;
pub(crate) mod provider;
pub(crate) mod search;
mod sort {
    use super::model::{SidebarFolder, SidebarThread};
    use std::sync::Arc;

    pub fn folder_sort_key(left: &SidebarFolder, right: &SidebarFolder) -> std::cmp::Ordering {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.label.cmp(&right.label))
    }

    pub fn thread_sort_key(
        left: &Arc<SidebarThread>,
        right: &Arc<SidebarThread>,
    ) -> std::cmp::Ordering {
        right
            .sort_timestamp()
            .cmp(&left.sort_timestamp())
            .then_with(|| right.is_live().cmp(&left.is_live()))
            .then_with(|| left.title.cmp(&right.title))
    }
}

pub use build::{build_sidebar_folders, thread_from_live_panel};
pub use display::clean_title;
pub use model::{
    SidebarFolder, SidebarFolderSummary, SidebarItem, SidebarThread, ThreadActivityOverride,
};
pub use search::build_visible_sidebar_items;
pub use sort::{folder_sort_key, thread_sort_key};
