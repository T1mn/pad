use crate::app::state::Mode;
use crate::app::App;
use crate::tree::FileTree;
use std::path::PathBuf;

impl App {
    pub fn toggle_tree(&mut self) {
        self.sidebar.show_tree = !self.sidebar.show_tree;
        self.focus_panel();
        if self.sidebar.show_tree {
            if let Some(thread) = self.selected_preview_thread() {
                let path = PathBuf::from(&thread.working_dir);
                if path.exists() {
                    self.sidebar.file_tree = Some(FileTree::new(path));
                    self.mode = Mode::Tree;
                    self.update_file_preview();
                }
            }
        } else {
            self.sidebar.file_tree = None;
            self.preview.file_preview_path = None;
            self.preview.file_preview_content.clear();
            self.mode = Mode::Normal;
        }
        self.dirty = true;
    }

    pub fn open_tree_in_home(&mut self) {
        if let Some(home) = dirs::home_dir() {
            self.sidebar.show_tree = true;
            self.focus_panel();
            self.sidebar.file_tree = Some(FileTree::new(home));
            self.mode = Mode::Tree;
            self.update_file_preview();
            self.dirty = true;
        }
    }

    pub fn close_tree(&mut self) {
        self.sidebar.show_tree = false;
        self.focus_panel();
        self.sidebar.file_tree = None;
        self.sidebar.agent_launcher = None;
        self.mode = Mode::Normal;
        self.dirty = true;
    }
}
