use super::App;
use crate::pad_sider::fs::{read_file_stats, relative_path_label, FileStats};
use std::path::{Path, PathBuf};

impl App {
    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.tree.get(self.selected).map(|row| &row.path)
    }

    pub fn selected_index_path(&self) -> Option<&PathBuf> {
        self.index_rows
            .get(self.index_selected)
            .map(|row| &row.path)
    }

    pub fn selected_is_dir(&self) -> bool {
        self.tree
            .get(self.selected)
            .map(|row| row.is_dir)
            .unwrap_or(false)
    }

    pub(crate) fn restore_selection(&mut self, selected_path: Option<&Path>) {
        if let Some(path) = selected_path {
            if self.set_selected_path(path) {
                return;
            }
        }
        if self.selected >= self.tree.len() {
            self.selected = self.tree.len().saturating_sub(1);
        }
    }

    pub(crate) fn set_selected_path(&mut self, path: &Path) -> bool {
        if let Some(index) = self.tree.iter().position(|row| row.path == path) {
            self.selected = index;
            return true;
        }
        false
    }

    pub(crate) fn refresh_selected(&mut self) {
        let Some(path) = self.selected_path().cloned() else {
            self.selected_label = ".".into();
            self.selected_stats = FileStats::default();
            return;
        };
        self.selected_label = relative_path_label(&self.cwd, &path);
        self.selected_stats = if path.is_file() {
            read_file_stats(&path)
        } else {
            FileStats::default()
        };
    }
}
