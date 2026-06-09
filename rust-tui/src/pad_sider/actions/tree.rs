use super::super::app::App;
use std::path::Path;

impl App {
    pub fn toggle_selected(&mut self) {
        let Some((is_dir, path)) = self
            .tree
            .get(self.selected)
            .map(|row| (row.is_dir, row.path.clone()))
        else {
            return;
        };
        if !is_dir || path == self.cwd {
            return;
        }
        if !self.expanded.insert(path.clone()) {
            self.expanded.remove(&path);
        }
        self.refresh();
        self.set_selected_path(&path);
        self.refresh_selected();
        self.refresh_file_preview();
    }

    pub fn reveal_path(&mut self, path: &Path) {
        if !path.starts_with(&self.cwd) {
            return;
        }

        self.expanded.insert(self.cwd.clone());
        let mut cursor = path.parent();
        while let Some(dir) = cursor {
            if !dir.starts_with(&self.cwd) {
                break;
            }
            self.expanded.insert(dir.to_path_buf());
            if dir == self.cwd {
                break;
            }
            cursor = dir.parent();
        }

        self.refresh();
        self.set_selected_path(path);
        self.refresh_selected();
        self.refresh_file_preview();
    }
}
