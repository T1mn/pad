use super::super::super::App;
use std::path::PathBuf;

impl App {
    pub fn update_tree_for_selection(&mut self) {
        if self.sidebar.show_tree {
            if let Some(thread) = self.selected_preview_thread() {
                let path = PathBuf::from(&thread.working_dir);
                if path.exists() {
                    let should_update = match &self.sidebar.file_tree {
                        None => true,
                        Some(tree) => tree.root_path != path,
                    };
                    if should_update {
                        self.sidebar.file_tree = Some(crate::tree::FileTree::new(path));
                        self.dirty = true;
                    }
                }
            }
        }
    }
}
