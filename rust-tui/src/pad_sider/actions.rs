use super::app::{App, Focus, NavMode};
use super::search::FileSearch;
use std::path::Path;

impl App {
    pub fn next(&mut self) {
        match self.focus {
            Focus::Tree => {
                if self.selected + 1 < self.tree.len() {
                    self.selected += 1;
                    self.refresh_selected();
                    self.refresh_file_preview();
                }
            }
            Focus::IndexMap => {
                if self.index_selected + 1 < self.index_rows.len() {
                    self.index_selected += 1;
                    self.refresh_file_preview();
                }
            }
            Focus::Preview => self.file_preview_scroll_down(1),
            Focus::Changes => self.changes_scroll = self.changes_scroll.saturating_add(1),
        }
    }

    pub fn previous(&mut self) {
        match self.focus {
            Focus::Tree => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.refresh_selected();
                    self.refresh_file_preview();
                }
            }
            Focus::IndexMap => {
                self.index_selected = self.index_selected.saturating_sub(1);
                self.refresh_file_preview();
            }
            Focus::Preview => self.file_preview_scroll_up(1),
            Focus::Changes => self.changes_scroll = self.changes_scroll.saturating_sub(1),
        }
    }

    pub fn toggle_selected(&mut self) {
        let Some(row) = self.tree.get(self.selected).cloned() else {
            return;
        };
        if !row.is_dir || row.path == self.cwd {
            return;
        }
        if !self.expanded.insert(row.path.clone()) {
            self.expanded.remove(&row.path);
        }
        self.refresh();
        self.set_selected_path(&row.path);
        self.refresh_selected();
        self.refresh_file_preview();
    }

    pub fn reset_position(&mut self) {
        match self.focus {
            Focus::Tree => {
                self.selected = 0;
                self.refresh_selected();
                self.refresh_file_preview();
            }
            Focus::IndexMap => {
                self.index_selected = 0;
                self.refresh_file_preview();
            }
            Focus::Preview => self.file_preview.scroll = 0,
            Focus::Changes => self.changes_scroll = 0,
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Tree | Focus::IndexMap => Focus::Preview,
            Focus::Preview => Focus::Changes,
            Focus::Changes => self.active_nav_focus(),
        };
    }

    pub fn focus_tree(&mut self) {
        self.set_tree_mode();
    }

    pub fn focus_preview(&mut self) {
        self.focus = Focus::Preview;
    }

    pub fn focus_changes(&mut self) {
        self.focus = Focus::Changes;
    }

    pub fn focus_active_nav(&mut self) {
        self.focus = self.active_nav_focus();
    }

    pub fn active_nav_weight(&self) -> u16 {
        match self.nav_mode {
            NavMode::Tree => self.layout_weights.tree,
            NavMode::IndexMap => self.layout_weights.index_map,
        }
    }

    pub fn active_nav_focus(&self) -> Focus {
        match self.nav_mode {
            NavMode::Tree => Focus::Tree,
            NavMode::IndexMap => Focus::IndexMap,
        }
    }

    pub fn open_search(&mut self) {
        self.search = Some(FileSearch::new(&self.cwd));
    }

    pub fn close_search(&mut self) {
        self.search = None;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn close_help(&mut self) {
        self.show_help = false;
    }

    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    pub fn zoom_text_in(&mut self) {
        self.text_zoom = (self.text_zoom + 1).min(2);
    }

    pub fn zoom_text_out(&mut self) {
        self.text_zoom = (self.text_zoom - 1).max(-1);
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

    pub fn jump_bottom(&mut self) {
        match self.focus {
            Focus::Tree => {
                self.selected = self.tree.len().saturating_sub(1);
                self.refresh_selected();
                self.refresh_file_preview();
            }
            Focus::IndexMap => {
                self.index_selected = self.index_rows.len().saturating_sub(1);
                self.refresh_file_preview();
            }
            Focus::Preview => self.file_preview.scroll = u16::MAX,
            Focus::Changes => self.changes_scroll = u16::MAX,
        }
    }
}

mod index_map;
mod layout;
mod nav_mode;
mod preview;

#[cfg(test)]
mod tests;
