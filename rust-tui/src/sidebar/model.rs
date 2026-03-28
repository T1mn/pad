use crate::model::{
    AgentState, AgentType, GitInfo, PreviewSessionOrigin, PreviewTurn, SessionCacheState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadRuntimeSource {
    Cli,
    App,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadActivityOverride {
    pub agent_type: AgentType,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub working_dir: String,
    pub state: AgentState,
    pub is_active: bool,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarThread {
    pub key: String,
    pub folder_key: String,
    pub working_dir: String,
    pub folder_label: String,
    pub agent_type: AgentType,
    pub runtime_source: Option<ThreadRuntimeSource>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub title: String,
    pub upstream_title: Option<String>,
    pub subtitle: Option<String>,
    pub title_override: Option<String>,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub updated_at: i64,
    pub live_pane_id: Option<String>,
    pub live_location: Option<String>,
    pub pid: Option<String>,
    pub git_info: Option<GitInfo>,
    pub state: AgentState,
    pub is_active: bool,
    pub cached_preview_turns: Vec<PreviewTurn>,
    pub session_cache_state: Option<SessionCacheState>,
    pub last_user_prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub has_unread_stop: bool,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarFolder {
    pub key: String,
    pub path: String,
    pub label: String,
    pub updated_at: i64,
    pub threads: Vec<SidebarThread>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarItem {
    Folder(SidebarFolder),
    Thread(SidebarThread),
}

impl SidebarThread {
    pub fn preview_origin(&self) -> Option<PreviewSessionOrigin> {
        if self.agent_type == AgentType::Codex {
            return Some(if self.live_pane_id.is_some() {
                PreviewSessionOrigin::Pane
            } else {
                PreviewSessionOrigin::App
            });
        }

        match self.runtime_source {
            Some(ThreadRuntimeSource::Cli) => Some(PreviewSessionOrigin::Pane),
            Some(ThreadRuntimeSource::App) => Some(PreviewSessionOrigin::App),
            None => None,
        }
    }

    pub fn is_live(&self) -> bool {
        self.live_pane_id.is_some()
    }
}

impl SidebarFolder {
    pub fn primary_thread(&self) -> Option<SidebarThread> {
        self.threads
            .iter()
            .find(|thread| thread.is_live() && thread.is_active)
            .or_else(|| self.threads.iter().find(|thread| thread.is_live()))
            .or_else(|| self.threads.iter().find(|thread| thread.is_active))
            .or_else(|| self.threads.first())
            .cloned()
    }
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

    pub fn as_folder(&self) -> Option<&SidebarFolder> {
        match self {
            SidebarItem::Folder(folder) => Some(folder),
            _ => None,
        }
    }

    pub fn as_thread(&self) -> Option<&SidebarThread> {
        match self {
            SidebarItem::Thread(thread) => Some(thread),
            _ => None,
        }
    }
}
