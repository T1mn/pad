use super::App;
use crate::log_debug;
use crate::model::AgentPanel;
use std::path::PathBuf;

impl App {
    pub fn next(&mut self) {
        if self.show_tree {
            if let Some(ref mut tree) = self.file_tree {
                log_debug!("nav: next (tree) selected={:?}", tree.state.selected());
                tree.next();
                self.dirty = true;
                return;
            }
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.filtered_panels().len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        log_debug!("nav: next (panel) index={}", i);
        self.table_state.select(Some(i));
        self.preview_pane_id = None;
        self.update_tree_for_selection();
        self.dirty = true;
    }

    pub fn previous(&mut self) {
        if self.show_tree {
            if let Some(ref mut tree) = self.file_tree {
                log_debug!("nav: previous (tree) selected={:?}", tree.state.selected());
                tree.previous();
                self.dirty = true;
                return;
            }
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_panels().len().saturating_sub(1)
                } else {
                    i.saturating_sub(1)
                }
            }
            None => 0,
        };
        log_debug!("nav: previous (panel) index={}", i);
        self.table_state.select(Some(i));
        self.preview_pane_id = None;
        self.update_tree_for_selection();
        self.dirty = true;
    }

    pub fn jump_to(&mut self, index: usize) {
        if index < self.filtered_panels().len() {
            self.table_state.select(Some(index));
            self.preview_pane_id = None;
            self.dirty = true;
        }
    }

    pub fn filtered_panels(&self) -> Vec<&AgentPanel> {
        if self.search_query.is_empty() {
            self.panels.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.panels
                .iter()
                .filter(|p| {
                    p.session.to_lowercase().contains(&query)
                        || p.window.to_lowercase().contains(&query)
                        || p.working_dir.to_lowercase().contains(&query)
                })
                .collect()
        }
    }

    pub fn selected_panel(&self) -> Option<&AgentPanel> {
        let filtered = self.filtered_panels();
        self.table_state
            .selected()
            .and_then(|i| filtered.get(i).copied())
    }

    pub fn update_tree_for_selection(&mut self) {
        if self.show_tree {
            if let Some(panel) = self.selected_panel() {
                let path = PathBuf::from(&panel.working_dir);
                if path.exists() {
                    let should_update = match &self.file_tree {
                        None => true,
                        Some(tree) => tree.root_path != path,
                    };
                    if should_update {
                        self.file_tree = Some(crate::tree::FileTree::new(path));
                        self.dirty = true;
                    }
                }
            }
        }
    }
}
