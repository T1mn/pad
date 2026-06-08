use crate::tree::{FileTree, TreeEntry};
use std::path::PathBuf;

impl FileTree {
    /// Get currently selected entry
    pub fn selected(&self) -> Option<&TreeEntry> {
        self.state
            .selected()
            .and_then(|index| self.entries.get(index))
    }

    /// Navigate into selected directory
    pub fn enter(&mut self) {
        let Some(entry) = self.selected_directory_info() else {
            return;
        };

        if entry.name == ".." {
            self.go_up();
        } else {
            self.enter_directory(entry.path);
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
        let Some(entry) = self.selected_directory_info() else {
            return;
        };
        if entry.name == ".." {
            return;
        }

        if self.expanded.contains(&entry.path) {
            self.expanded.remove(&entry.path);
        } else {
            self.enter_directory(entry.path);
        }
    }

    fn selected_directory_info(&self) -> Option<SelectedDirectory> {
        self.selected()
            .filter(|entry| entry.is_dir)
            .map(|entry| SelectedDirectory {
                name: entry.name.clone(),
                path: entry.path.clone(),
            })
    }

    fn enter_directory(&mut self, path: PathBuf) {
        self.expanded.insert(path.clone());
        self.current_path = path;
        self.refresh_entries();
        self.state.select(Some(0));
    }
}

struct SelectedDirectory {
    name: String,
    path: PathBuf,
}
