use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// File tree entry
#[derive(Clone, Debug)]
pub struct TreeEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub depth: usize,
    pub expanded: bool,
    pub has_children: bool,
}

/// File tree explorer state
pub struct FileTree {
    pub root_path: PathBuf,
    pub entries: Vec<TreeEntry>,
    pub state: ListState,
    pub expanded: HashSet<PathBuf>,
}

impl FileTree {
    pub fn new(root_path: PathBuf) -> Self {
        let mut tree = Self {
            root_path: root_path.clone(),
            entries: Vec::new(),
            state: ListState::default(),
            expanded: HashSet::new(),
        };
        tree.refresh();
        tree.state.select(Some(0));
        tree
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        let root = self.root_path.clone();
        self.scan_directory(&root, 0);
    }

    fn scan_directory(&mut self, path: &Path, depth: usize) {
        if let Ok(entries) = std::fs::read_dir(path) {
            let mut dir_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            
            // Sort: directories first, then by name
            dir_entries.sort_by(|a, b| {
                let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });

            for entry in dir_entries {
                let name = entry.file_name().to_string_lossy().to_string();
                let entry_path = entry.path();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                
                // Skip hidden files and common non-code directories
                if name.starts_with('.') && matches!(name.as_str(), ".git" | ".svn" | ".hg") {
                    continue;
                }
                if matches!(name.as_str(), "node_modules" | "target" | "__pycache__" | "dist" | "build") {
                    continue;
                }

                let has_children = if is_dir {
                    std::fs::read_dir(&entry_path)
                        .map(|mut d| d.next().is_some())
                        .unwrap_or(false)
                } else {
                    false
                };

                let expanded = is_dir && self.expanded.contains(&entry_path);

                self.entries.push(TreeEntry {
                    name,
                    path: entry_path.clone(),
                    is_dir,
                    depth,
                    expanded,
                    has_children,
                });

                // Recursively add children if expanded
                if expanded {
                    self.scan_directory(&entry_path, depth + 1);
                }
            }
        }
    }

    pub fn toggle_selected(&mut self) {
        if let Some(idx) = self.state.selected() {
            if let Some(entry) = self.entries.get(idx) {
                if entry.is_dir {
                    if self.expanded.contains(&entry.path) {
                        self.expanded.remove(&entry.path);
                    } else {
                        self.expanded.insert(entry.path.clone());
                    }
                    self.refresh();
                }
            }
        }
    }

    pub fn select_next(&mut self) {
        let idx = self.state.selected().unwrap_or(0);
        if idx < self.entries.len().saturating_sub(1) {
            self.state.select(Some(idx + 1));
        }
    }

    pub fn select_previous(&mut self) {
        let idx = self.state.selected().unwrap_or(0);
        if idx > 0 {
            self.state.select(Some(idx - 1));
        }
    }

    pub fn get_selected(&self) -> Option<&TreeEntry> {
        self.state.selected().and_then(|idx| self.entries.get(idx))
    }

    pub fn get_selected_path(&self) -> Option<PathBuf> {
        self.get_selected().map(|e| e.path.clone())
    }

    fn file_icon(filename: &str, is_dir: bool) -> &'static str {
        if is_dir {
            return "📁";
        }
        let ext = filename.split('.').last().unwrap_or("");
        match ext {
            "rs" => "🦀",
            "py" => "🐍",
            "js" | "ts" | "jsx" | "tsx" => "📜",
            "go" => "🔵",
            "java" => "☕",
            "c" | "cpp" | "h" | "hpp" => "🔧",
            "md" | "mdx" => "📝",
            "json" | "yaml" | "yml" | "toml" => "⚙️",
            "sh" | "bash" | "zsh" => "🐚",
            "html" | "css" => "🌐",
            _ => "📄",
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, title: &str) {
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .enumerate()
            .map(|(_idx, entry)| {
                let icon = Self::file_icon(&entry.name, entry.is_dir);
                let indent = "  ".repeat(entry.depth);
                let expand_icon = if entry.is_dir {
                    if entry.expanded { "▼ " } else { "▶ " }
                } else {
                    "  "
                };
                
                let content = format!("{}{}{} {}", indent, expand_icon, icon, entry.name);
                let style = if entry.is_dir {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                
                ListItem::new(Line::from(vec![Span::styled(content, style)]))
            })
            .collect();

        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL);

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.state);
    }
}
