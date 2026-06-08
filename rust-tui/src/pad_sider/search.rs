mod filter;
mod input;

use super::fs::relative_path_label;
use super::tree::scan_files;
use crossterm::event::KeyEvent;
use std::path::{Path, PathBuf};

pub enum SearchAction {
    None,
    Cancel,
    Submit(PathBuf),
}

struct SearchItem {
    path: PathBuf,
    relative: String,
}

pub struct FileSearch {
    items: Vec<SearchItem>,
    filtered: Vec<(usize, u32)>,
    query: String,
    selected: usize,
}

impl FileSearch {
    pub fn new(root: &Path) -> Self {
        let items = scan_files(root)
            .into_iter()
            .map(|path| SearchItem {
                relative: relative_path_label(root, &path),
                path,
            })
            .collect::<Vec<_>>();
        let filtered = filter::initial_filter(&items);
        Self {
            items,
            filtered,
            query: String::new(),
            selected: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SearchAction {
        input::handle_key(self, key)
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn relative_at(&self, index: usize) -> Option<&str> {
        self.filtered
            .get(index)
            .and_then(|(item_index, _)| self.items.get(*item_index))
            .map(|item| item.relative.as_str())
    }

    fn selected_path(&self) -> Option<&Path> {
        self.filtered
            .get(self.selected)
            .and_then(|(item_index, _)| self.items.get(*item_index))
            .map(|item| item.path.as_path())
    }
}

#[cfg(test)]
mod tests;
