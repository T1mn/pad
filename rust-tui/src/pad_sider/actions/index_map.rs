use super::super::app::App;
use super::super::fs::read_markdown_file;
use super::super::preview::MarkdownPreview;
use std::path::PathBuf;

impl App {
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

    pub fn open_selected_index_preview(&mut self) {
        let Some(path) = self.selected_index_path().cloned() else {
            return;
        };
        self.preview = Some(MarkdownPreview {
            content: read_markdown_file(&path),
            path,
            scroll: 0,
        });
    }

    pub fn reveal_selected_index_in_tree(&mut self) {
        let Some(path) = self.selected_index_path().cloned() else {
            return;
        };
        self.reveal_path(&path);
        self.set_tree_mode();
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
