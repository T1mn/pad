mod agent_launcher {
    use std::path::PathBuf;

    const DEFAULT_AGENTS: &[(&str, &str)] = &[
        ("claude-code", "claude"),
        ("codex", "codex"),
        ("grok-build", "grok"),
        ("kimi-cli", "kimi-cli"),
        ("gemini-cli", "gemini-cli"),
        ("opencode", "opencode"),
        ("aider", "aider"),
        ("cursor", "cursor"),
    ];

    /// Agent launcher state.
    pub struct AgentLauncher {
        pub selected: usize,
        pub agents: Vec<(String, String)>,
        pub target_dir: PathBuf,
    }

    impl AgentLauncher {
        pub fn with_agents(target_dir: PathBuf, agents: Vec<(String, String)>) -> Self {
            let agents = if agents.is_empty() {
                default_agents()
            } else {
                agents
            };
            Self {
                selected: 0,
                agents,
                target_dir,
            }
        }

        pub fn next(&mut self) {
            if self.selected < self.agents.len().saturating_sub(1) {
                self.selected += 1;
            }
        }

        pub fn previous(&mut self) {
            if self.selected > 0 {
                self.selected -= 1;
            }
        }

        pub fn selected_agent(&self) -> Option<&(String, String)> {
            self.agents.get(self.selected)
        }
    }

    fn default_agents() -> Vec<(String, String)> {
        DEFAULT_AGENTS
            .iter()
            .map(|(name, command)| ((*name).to_string(), (*command).to_string()))
            .collect()
    }
}
mod navigation;
mod preview_type;
mod render;
mod search {
    use super::{FileTree, TreeMode};
    use crate::text_match::contains_ignore_case;

    impl FileTree {
        /// Activate search mode
        pub fn start_search(&mut self) {
            self.mode = TreeMode::Search;
            self.search_query.clear();
        }

        /// Cancel search
        pub fn cancel_search(&mut self) {
            self.mode = TreeMode::Normal;
            self.search_query.clear();
            self.refresh_entries(); // Show all entries again
        }

        /// Add character to search query
        pub fn search_input(&mut self, c: char) {
            if self.mode == TreeMode::Search {
                self.search_query.push(c);
                self.filter_entries();
            }
        }

        /// Remove last character from search query
        pub fn search_backspace(&mut self) {
            if self.mode == TreeMode::Search {
                self.search_query.pop();
                if self.search_query.is_empty() {
                    self.refresh_entries();
                } else {
                    self.filter_entries();
                }
            }
        }

        /// Clear the search query while staying in search mode
        pub fn clear_search_query(&mut self) {
            if self.mode == TreeMode::Search {
                self.search_query.clear();
                self.refresh_entries();
            }
        }

        /// Filter entries based on search query
        fn filter_entries(&mut self) {
            self.entries = self
                .scan_directory(&self.current_path)
                .into_iter()
                .filter(|entry| {
                    // Always keep ".."
                    if entry.name == ".." {
                        return true;
                    }
                    contains_ignore_case(&entry.name, &self.search_query)
                })
                .collect();

            // Reset selection
            self.state.select(Some(0));
        }
    }
}

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
