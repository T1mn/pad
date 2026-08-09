mod scan {
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
}
mod selection {
    use crate::tree::FileTree;

    impl FileTree {
        /// Select next entry
        pub fn next(&mut self) {
            let count = self.entries.len();
            if count == 0 {
                return;
            }
            let index = self.state.selected().unwrap_or(0);
            if index < count - 1 {
                self.state.select(Some(index + 1));
            }
        }

        /// Select previous entry
        pub fn previous(&mut self) {
            let index = self.state.selected().unwrap_or(0);
            if index > 0 {
                self.state.select(Some(index - 1));
            }
        }
    }
}
mod travel {
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
}
