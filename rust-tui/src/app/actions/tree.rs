mod control {
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
}
mod launcher {
    use crate::app::state::Mode;
    use crate::app::App;
    use crate::fuzzy::{scan_directories, FuzzyPicker};
    use crate::tree::AgentLauncher;
    use std::path::PathBuf;

    impl App {
        pub fn open_agent_launcher(&mut self, target_dir: PathBuf) {
            let agent_tuples: Vec<(String, String)> = self
                .config
                .agents
                .iter()
                .map(|a| (a.name.clone(), a.cmd.clone()))
                .collect();
            self.sidebar.agent_launcher =
                Some(AgentLauncher::with_agents(target_dir, agent_tuples));
            self.mode = Mode::AgentLauncher;
            self.dirty = true;
        }

        pub fn close_agent_launcher(&mut self) {
            let was_fuzzy = self.fuzzy_from_normal;
            self.sidebar.agent_launcher = None;
            self.fuzzy_from_normal = false;
            if was_fuzzy || !self.sidebar.show_tree {
                self.mode = Mode::Normal;
            } else {
                self.mode = Mode::Tree;
            }
            self.dirty = true;
        }

        pub fn open_fuzzy_picker(&mut self) {
            let home = dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());
            let items = scan_directories(&home, 3);
            self.fuzzy_picker = Some(FuzzyPicker::new(items));
            self.fuzzy_from_normal = true;
            self.mode = Mode::FuzzyPicker;
            self.dirty = true;
        }

        pub fn close_fuzzy_picker(&mut self) {
            self.fuzzy_picker = None;
            self.fuzzy_from_normal = false;
            self.mode = Mode::Normal;
            self.dirty = true;
        }
    }
}
mod preview;
