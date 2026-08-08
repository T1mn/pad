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
mod preview_type {
    use std::path::Path;

    const MARKDOWN_SUFFIXES: &[&str] = &[".md", ".markdown"];
    const IMAGE_SUFFIXES: &[&str] = &[".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp"];
    const BINARY_SUFFIXES: &[&str] = &[".exe", ".dll", ".so", ".dylib", ".bin"];
    const TEXT_SUFFIXES: &[&str] = &[
        ".rs",
        ".py",
        ".js",
        ".ts",
        ".go",
        ".java",
        ".c",
        ".cpp",
        ".h",
        ".hpp",
        ".rb",
        ".php",
        ".swift",
        ".kt",
        ".scala",
        ".r",
        ".sh",
        ".bash",
        ".zsh",
        ".fish",
        ".json",
        ".toml",
        ".yaml",
        ".yml",
        ".xml",
        ".html",
        ".css",
        ".sql",
        ".txt",
        ".log",
        ".conf",
        ".config",
        ".ini",
        ".env",
        ".gitignore",
        ".dockerignore",
    ];

    /// File preview type
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum PreviewType {
        Text,      // Source code, config files
        Markdown,  // Markdown files
        Image,     // Image files (PNG, JPG, etc)
        Binary,    // Binary files (cannot preview)
        Directory, // Directory
        Unknown,   // Unknown type
    }

    impl PreviewType {
        /// Detect preview type from file path
        pub fn from_path(path: &Path) -> Self {
            if path.is_dir() {
                return PreviewType::Directory;
            }

            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if has_any_suffix(name, MARKDOWN_SUFFIXES) {
                PreviewType::Markdown
            } else if has_any_suffix(name, IMAGE_SUFFIXES) {
                PreviewType::Image
            } else if has_any_suffix(name, BINARY_SUFFIXES) {
                PreviewType::Binary
            } else if has_any_suffix(name, TEXT_SUFFIXES) {
                PreviewType::Text
            } else {
                PreviewType::Unknown
            }
        }

        /// Check if file can be previewed as text
        pub fn is_text(&self) -> bool {
            matches!(self, PreviewType::Text | PreviewType::Markdown)
        }

        /// Check if file is an image
        pub fn is_image(&self) -> bool {
            matches!(self, PreviewType::Image)
        }
    }

    fn has_any_suffix(name: &str, suffixes: &[&str]) -> bool {
        suffixes
            .iter()
            .any(|suffix| has_suffix_ignore_ascii_case(name, suffix))
    }

    fn has_suffix_ignore_ascii_case(name: &str, suffix: &str) -> bool {
        let name = name.as_bytes();
        let suffix = suffix.as_bytes();
        if name.len() < suffix.len() {
            return false;
        }
        name[name.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
    }
}
mod render {
    use super::{FileTree, TreeEntry};
    use crate::theme::Theme;
    use ratatui::{
        layout::Rect,
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, List, ListItem},
        Frame,
    };

    impl FileTree {
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
                } else if name.ends_with(".sh") || name.ends_with(".bash") || name.ends_with(".zsh")
                {
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
}
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
