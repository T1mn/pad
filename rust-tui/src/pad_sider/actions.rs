use super::app::{App, Focus, MarkdownPreview};
use super::fs::{is_markdown_file, read_markdown_file};
use super::search::FileSearch;
use std::path::Path;

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

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Tree => Focus::Changes,
            Focus::Changes => Focus::Tree,
        };
    }

    pub fn open_search(&mut self) {
        self.search = Some(FileSearch::new(&self.cwd));
    }

    pub fn close_search(&mut self) {
        self.search = None;
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
}

#[cfg(test)]
mod tests {
    use super::App;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reveal_path_expands_parents_and_selects_file() {
        let root = temp_dir("reveal_path_expands_parents_and_selects_file");
        let target = root.join("docs/guide/readme.md");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "# guide").unwrap();

        let mut app = App::new(root.clone(), None);
        app.reveal_path(&target);

        assert_eq!(app.selected_path(), Some(&target));
        assert!(app.expanded.contains(&root.join("docs")));
        assert!(app.expanded.contains(&root.join("docs/guide")));

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pad_sider_{name}_{unique}"))
    }
}
