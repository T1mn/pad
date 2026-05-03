use super::app::{App, Focus, MarkdownPreview};
use super::fs::{is_markdown_file, read_markdown_file};
use super::search::FileSearch;
use std::path::{Path, PathBuf};

impl App {
    pub fn next(&mut self) {
        match self.focus {
            Focus::Tree => {
                if self.selected + 1 < self.tree.len() {
                    self.selected += 1;
                    self.refresh_selected();
                }
            }
            Focus::Changes => self.changes_scroll = self.changes_scroll.saturating_add(1),
        }
    }

    pub fn previous(&mut self) {
        match self.focus {
            Focus::Tree => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.refresh_selected();
                }
            }
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
    }

    pub fn open_preview(&mut self) {
        let Some(path) = self.selected_path().cloned() else {
            return;
        };
        if !is_markdown_file(&path) {
            return;
        }
        self.preview = Some(MarkdownPreview {
            content: read_markdown_file(&path),
            path,
            scroll: 0,
        });
    }

    pub fn close_preview(&mut self) {
        self.preview = None;
    }

    pub fn preview_down(&mut self) {
        if let Some(preview) = self.preview.as_mut() {
            preview.scroll = preview.scroll.saturating_add(1);
        }
    }

    pub fn preview_up(&mut self) {
        if let Some(preview) = self.preview.as_mut() {
            preview.scroll = preview.scroll.saturating_sub(1);
        }
    }

    pub fn reset_position(&mut self) {
        match self.focus {
            Focus::Tree => {
                self.selected = 0;
                self.refresh_selected();
            }
            Focus::Changes => self.changes_scroll = 0,
        }
    }

    pub fn reset_preview(&mut self) {
        if let Some(preview) = self.preview.as_mut() {
            preview.scroll = 0;
        }
    }

    pub fn preview_bottom(&mut self) {
        if let Some(preview) = self.preview.as_mut() {
            preview.scroll = u16::MAX;
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Tree => Focus::Changes,
            Focus::Changes => Focus::Tree,
        };
    }

    pub fn focus_tree(&mut self) {
        self.focus = Focus::Tree;
    }

    pub fn focus_changes(&mut self) {
        self.focus = Focus::Changes;
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
    }

    pub fn jump_bottom(&mut self) {
        match self.focus {
            Focus::Tree => {
                self.selected = self.tree.len().saturating_sub(1);
                self.refresh_selected();
            }
            Focus::Changes => self.changes_scroll = u16::MAX,
        }
    }

    pub fn open_nearest_index_preview(&mut self) {
        let Some(path) = self.nearest_index_path() else {
            return;
        };
        self.reveal_path(&path);
        self.preview = Some(MarkdownPreview {
            content: read_markdown_file(&path),
            path,
            scroll: 0,
        });
    }

    fn nearest_index_path(&self) -> Option<PathBuf> {
        let selected = self.selected_path()?;
        let mut cursor = if selected.is_dir() {
            selected.as_path()
        } else {
            selected.parent()?
        };

        loop {
            if !cursor.starts_with(&self.cwd) {
                return None;
            }
            let candidate = cursor.join("index.md");
            if candidate.is_file() {
                return Some(candidate);
            }
            if cursor == self.cwd {
                return None;
            }
            cursor = cursor.parent()?;
        }
    }
}

#[cfg(test)]
mod tests;
