mod agent_launcher;
mod navigation;
mod preview_type;
mod render;
mod search;

pub use agent_launcher::AgentLauncher;
pub use preview_type::PreviewType;

use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::path::PathBuf;

/// Tree view mode
#[derive(Clone, Copy, PartialEq)]
pub enum TreeMode {
    Normal,
    Search,
}

/// File tree explorer state
pub struct FileTree {
    /// Root path of the tree
    pub root_path: PathBuf,
    /// Current directory being viewed
    pub current_path: PathBuf,
    /// Entries in current directory
    pub entries: Vec<TreeEntry>,
    /// List state for selection
    pub state: ListState,
    /// Set of expanded directories
    pub expanded: HashSet<PathBuf>,
    /// Search query
    pub search_query: String,
    /// Current mode
    pub mode: TreeMode,
}

/// Single entry in tree
#[derive(Clone, Debug)]
pub struct TreeEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_expanded: bool,
}

impl FileTree {
    /// Create new file tree starting at given path
    pub fn new(start_path: PathBuf) -> Self {
        let mut tree = Self {
            root_path: start_path.clone(),
            current_path: start_path.clone(),
            entries: Vec::new(),
            state: ListState::default(),
            expanded: HashSet::new(),
            search_query: String::new(),
            mode: TreeMode::Normal,
        };
        tree.refresh_entries();
        tree.state.select(Some(0));
        tree
    }
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
