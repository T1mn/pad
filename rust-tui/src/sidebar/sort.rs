use super::model::{SidebarFolder, SidebarThread};

pub fn folder_sort_key(left: &SidebarFolder, right: &SidebarFolder) -> std::cmp::Ordering {
    right
        .updated_at
        .cmp(&left.updated_at)
        .then_with(|| left.label.cmp(&right.label))
}

pub fn thread_sort_key(left: &SidebarThread, right: &SidebarThread) -> std::cmp::Ordering {
    right
        .sort_timestamp()
        .cmp(&left.sort_timestamp())
        .then_with(|| right.is_live().cmp(&left.is_live()))
        .then_with(|| left.title.cmp(&right.title))
}
