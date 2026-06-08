use super::folder::SidebarFolderSummary;
use super::thread::SidebarThread;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarItem {
    Folder(Box<SidebarFolderSummary>),
    Thread(Arc<SidebarThread>),
}

impl SidebarItem {
    pub fn folder(folder: SidebarFolderSummary) -> Self {
        SidebarItem::Folder(Box::new(folder))
    }

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
            SidebarItem::Folder(folder) => Some(folder.as_ref()),
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
