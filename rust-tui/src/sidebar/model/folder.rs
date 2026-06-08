use super::thread::SidebarThread;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarFolder {
    pub key: String,
    pub path: String,
    pub label: String,
    pub updated_at: i64,
    pub threads: Vec<Arc<SidebarThread>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarFolderSummary {
    pub key: String,
    pub path: String,
    pub label: String,
    pub updated_at: i64,
    pub thread_count: usize,
    pub has_unread_stop: bool,
}

impl SidebarFolder {
    pub fn summary(&self) -> SidebarFolderSummary {
        SidebarFolderSummary {
            key: self.key.clone(),
            path: self.path.clone(),
            label: self.label.clone(),
            updated_at: self.updated_at,
            thread_count: self.threads.len(),
            has_unread_stop: self.threads.iter().any(|thread| thread.has_unread_stop),
        }
    }

    pub fn primary_thread(&self) -> Option<SidebarThread> {
        self.threads
            .iter()
            .find(|thread| thread.is_live() && thread.is_active)
            .or_else(|| self.threads.iter().find(|thread| thread.is_live()))
            .or_else(|| self.threads.iter().find(|thread| thread.is_active))
            .or_else(|| self.threads.first())
            .map(|thread| thread.as_ref().clone())
    }
}
