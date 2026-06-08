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
        self.sidebar.agent_launcher = Some(AgentLauncher::with_agents(target_dir, agent_tuples));
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
