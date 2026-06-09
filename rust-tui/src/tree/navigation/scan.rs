use crate::tree::{FileTree, TreeEntry};
use std::path::Path;

impl FileTree {
    /// Refresh entries for current directory
    pub fn refresh_entries(&mut self) {
        self.entries = self.scan_directory(&self.current_path);
        self.keep_selection_valid();
    }

    /// Scan directory and return entries
    pub(in crate::tree) fn scan_directory(&self, path: &Path) -> Vec<TreeEntry> {
        let mut entries = Vec::new();
        if let Some(parent) = parent_entry(path, &self.root_path) {
            entries.push(parent);
        }
        let Ok(dir_entries) = std::fs::read_dir(path) else {
            return entries;
        };

        let mut items: Vec<_> = dir_entries.filter_map(|entry| entry.ok()).collect();
        items.sort_by(compare_dir_entries);

        entries.extend(
            items
                .into_iter()
                .filter_map(|entry| self.tree_entry_for_dir_entry(entry)),
        );
        entries
    }

    fn keep_selection_valid(&mut self) {
        let count = self.entries.len();
        if count == 0 {
            self.state.select(None);
            return;
        }

        let current = self.state.selected().unwrap_or(0);
        if current >= count {
            self.state.select(Some(count - 1));
        }
    }

    fn tree_entry_for_dir_entry(&self, entry: std::fs::DirEntry) -> Option<TreeEntry> {
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_tree_entry(&name) {
            return None;
        }

        let path = entry.path();
        let is_dir = entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);
        let is_expanded = is_dir && self.expanded.contains(&path);
        Some(TreeEntry {
            name,
            path,
            is_dir,
            is_expanded,
        })
    }
}

fn parent_entry(path: &Path, root_path: &Path) -> Option<TreeEntry> {
    if path == root_path {
        return None;
    }

    let parent = path.parent()?;
    Some(TreeEntry {
        name: "..".to_string(),
        path: parent.to_path_buf(),
        is_dir: true,
        is_expanded: false,
    })
}

fn compare_dir_entries(a: &std::fs::DirEntry, b: &std::fs::DirEntry) -> std::cmp::Ordering {
    let a_is_dir = a
        .file_type()
        .map(|file_type| file_type.is_dir())
        .unwrap_or(false);
    let b_is_dir = b
        .file_type()
        .map(|file_type| file_type.is_dir())
        .unwrap_or(false);
    match (a_is_dir, b_is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.file_name().cmp(&b.file_name()),
    }
}

fn should_skip_tree_entry(name: &str) -> bool {
    (name.starts_with('.') && matches!(name, ".git" | ".svn" | ".hg"))
        || matches!(
            name,
            "node_modules" | "target" | "__pycache__" | "dist" | "build"
        )
}
