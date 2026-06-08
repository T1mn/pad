use super::folder::SidebarFolderSummary;
use super::thread::SidebarThread;
use std::sync::Arc;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarItem {
    Folder(SidebarFolderSummary),
    Thread(Arc<SidebarThread>),
}

impl SidebarItem {
    pub fn key(&self) -> &str {
        match self {
            SidebarItem::Folder(folder) => &folder.key,
            SidebarItem::Thread(thread) => &thread.key,
        }
    }

    pub fn folder_key(&self) -> &str {
        match self {
            SidebarItem::Folder(folder) => &folder.key,
            SidebarItem::Thread(thread) => &thread.folder_key,
        }
    }

    pub fn as_folder(&self) -> Option<&SidebarFolderSummary> {
        match self {
            SidebarItem::Folder(folder) => Some(folder),
            _ => None,
        }
    }

    pub fn as_thread(&self) -> Option<&SidebarThread> {
        match self {
            SidebarItem::Thread(thread) => Some(thread.as_ref()),
            _ => None,
        }
    }
}
