mod agent_launcher;
mod preview_type;

pub use agent_launcher::AgentLauncher;
pub use preview_type::PreviewType;

use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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

    /// Refresh entries for current directory
    pub fn refresh_entries(&mut self) {
        self.entries = self.scan_directory(&self.current_path);
        // Keep selection valid
        let count = self.entries.len();
        if count == 0 {
            self.state.select(None);
        } else {
            let current = self.state.selected().unwrap_or(0);
            if current >= count {
                self.state.select(Some(count - 1));
            }
        }
    }

    /// Scan directory and return entries
    fn scan_directory(&self, path: &Path) -> Vec<TreeEntry> {
        let mut entries = Vec::new();

        // Add ".." if not at root
        if path != self.root_path {
            if let Some(parent) = path.parent() {
                entries.push(TreeEntry {
                    name: "..".to_string(),
                    path: parent.to_path_buf(),
                    is_dir: true,
                    is_expanded: false,
                });
            }
        }

        if let Ok(dir_entries) = std::fs::read_dir(path) {
            let mut items: Vec<_> = dir_entries.filter_map(|e| e.ok()).collect();

            // Sort: directories first, then by name
            items.sort_by(|a, b| {
                let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });

            for entry in items {
                let name = entry.file_name().to_string_lossy().to_string();
                let entry_path = entry.path();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

                // Skip hidden files and common build directories
                if name.starts_with('.') && matches!(name.as_str(), ".git" | ".svn" | ".hg") {
                    continue;
                }
                if matches!(
                    name.as_str(),
                    "node_modules" | "target" | "__pycache__" | "dist" | "build"
                ) {
                    continue;
                }

                let is_expanded = is_dir && self.expanded.contains(&entry_path);

                entries.push(TreeEntry {
                    name,
                    path: entry_path,
                    is_dir,
                    is_expanded,
                });
            }
        }

        entries
    }

    /// Get currently selected entry
    pub fn selected(&self) -> Option<&TreeEntry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    /// Navigate into selected directory
    pub fn enter(&mut self) {
        let entry_info = self
            .selected()
            .map(|e| (e.is_dir, e.name.clone(), e.path.clone()));
        if let Some((is_dir, name, path)) = entry_info {
            if is_dir {
                if name == ".." {
                    self.go_up();
                } else {
                    self.expanded.insert(path.clone());
                    self.current_path = path;
                    self.refresh_entries();
                    self.state.select(Some(0));
                }
            }
        }
    }

    /// Go to parent directory
    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            if parent.starts_with(&self.root_path) {
                self.current_path = parent.to_path_buf();
                self.refresh_entries();
                self.state.select(Some(0));
            }
        }
    }

    /// Toggle directory expansion (for in-place expansion, currently not used)
    pub fn toggle(&mut self) {
        let entry_info = self
            .selected()
            .map(|e| (e.is_dir, e.name.clone(), e.path.clone()));
        if let Some((is_dir, name, path)) = entry_info {
            if is_dir && name != ".." {
                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path.clone());
                    // Enter the directory
                    self.current_path = path;
                    self.refresh_entries();
                    self.state.select(Some(0));
                }
            }
        }
    }

    /// Select next entry
    pub fn next(&mut self) {
        let count = self.entries.len();
        if count == 0 {
            return;
        }
        let i = self.state.selected().unwrap_or(0);
        if i < count - 1 {
            self.state.select(Some(i + 1));
        }
    }

    /// Select previous entry
    pub fn previous(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        if i > 0 {
            self.state.select(Some(i - 1));
        }
    }

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
        let query = self.search_query.to_lowercase();
        let all_entries = self.scan_directory(&self.current_path);

        self.entries = all_entries
            .into_iter()
            .filter(|e| {
                // Always keep ".."
                if e.name == ".." {
                    return true;
                }
                e.name.to_lowercase().contains(&query)
            })
            .collect();

        // Reset selection
        self.state.select(Some(0));
    }

    /// Get icon for file type
    fn file_icon(entry: &TreeEntry) -> &'static str {
        if entry.is_dir {
            if entry.name == ".." {
                "⬆️"
            } else if entry.is_expanded {
                "📂"
            } else {
                "📁"
            }
        } else {
            let name = &entry.name;
            if name.ends_with(".rs") {
                "🦀"
            } else if name.ends_with(".py") {
                "🐍"
            } else if name.ends_with(".js") || name.ends_with(".ts") {
                "📜"
            } else if name.ends_with(".go") {
                "🔵"
            } else if name.ends_with(".java") {
                "☕"
            } else if name.ends_with(".md") {
                "📝"
            } else if name.ends_with(".json")
                || name.ends_with(".toml")
                || name.ends_with(".yaml")
                || name.ends_with(".yml")
            {
                "⚙️"
            } else if name.ends_with(".sh") || name.ends_with(".bash") || name.ends_with(".zsh") {
                "🐚"
            } else if name.ends_with(".html") || name.ends_with(".css") {
                "🌐"
            } else {
                "📄"
            }
        }
    }

    /// Render tree view
    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Create list items
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let icon = Self::file_icon(entry);
                let content = format!("{} {}", icon, entry.name);

                let style = if entry.is_dir {
                    Style::default().fg(theme.accent)
                } else {
                    Style::default().fg(theme.fg)
                };

                ListItem::new(Line::from(vec![Span::styled(content, style)]))
            })
            .collect();

        // Create block with title
        let title = format!("📁 {}", self.current_path.display());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_focused));

        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(list, area, &mut self.state);
    }
}
