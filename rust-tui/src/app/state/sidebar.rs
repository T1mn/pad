use crate::model::AgentPanel;
use crate::sidebar::{SidebarFolder, SidebarItem, SidebarThread, ThreadActivityOverride};
use crate::tree;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadActionKind {
    Archive,
    Unarchive,
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

pub struct SidebarState {
    pub show_tree: bool,
    pub file_tree: Option<tree::FileTree>,
    pub agent_launcher: Option<tree::AgentLauncher>,
    pub delete_target: Option<AgentPanel>,
    pub pending_thread_action: Option<PendingThreadAction>,
    pub thread_meta_editing: bool,
    pub thread_meta_edit_kind: ThreadMetaEditKind,
    pub thread_meta_target: Option<SidebarThread>,
    pub thread_meta_buffer: String,
    pub expanded_folders: HashSet<String>,
    pub hovered_folder_key: Option<String>,
    pub selected_sidebar_key: Option<String>,
    pub pending_sidebar_selection_index: Option<usize>,
    pub pending_space_action: Option<PendingSidebarSpaceAction>,
    pub archived_threads_view: bool,
    pub display_session_scope: String,
    pub app_thread_activity: HashMap<String, ThreadActivityOverride>,
    pub thread_sort_activity: HashMap<String, i64>,
    pub startup_thread_sort_activity: HashMap<String, i64>,
    pub startup_thread_sort_seeded: bool,
    pub sidebar_folders_cache: Vec<SidebarFolder>,
    pub visible_sidebar_items_cache: Vec<SidebarItem>,
    pub sidebar_folders_dirty: bool,
    pub visible_sidebar_items_dirty: bool,
}

impl SidebarState {
    pub fn new(display_session_scope: String) -> Self {
        Self {
            show_tree: false,
            file_tree: None,
            agent_launcher: None,
            delete_target: None,
            pending_thread_action: None,
            thread_meta_editing: false,
            thread_meta_edit_kind: ThreadMetaEditKind::Title,
            thread_meta_target: None,
            thread_meta_buffer: String::new(),
            expanded_folders: HashSet::new(),
            hovered_folder_key: None,
            selected_sidebar_key: None,
            pending_sidebar_selection_index: None,
            pending_space_action: None,
            archived_threads_view: false,
            display_session_scope,
            app_thread_activity: HashMap::new(),
            thread_sort_activity: HashMap::new(),
            startup_thread_sort_activity: HashMap::new(),
            startup_thread_sort_seeded: false,
            sidebar_folders_cache: Vec::new(),
            visible_sidebar_items_cache: Vec::new(),
            sidebar_folders_dirty: true,
            visible_sidebar_items_dirty: true,
        }
    }

    #[allow(dead_code)]
    pub fn sidebar_folders_ref(&self) -> &[SidebarFolder] {
        &self.sidebar_folders_cache
    }

    #[allow(dead_code)]
    pub fn visible_sidebar_items_ref(&self) -> &[SidebarItem] {
        &self.visible_sidebar_items_cache
    }
}

impl Default for SidebarState {
    fn default() -> Self {
        Self::new(String::from("live"))
    }
}
