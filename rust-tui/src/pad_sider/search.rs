mod filter {
    use super::{FileSearch, SearchItem};
    use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
    use nucleo_matcher::{Matcher, Utf32Str};

    pub(super) fn initial_filter(items: &[SearchItem]) -> Vec<(usize, u32)> {
        let mut filtered = Vec::with_capacity(items.len());
        fill_initial_filter(&mut filtered, items.len());
        filtered
    }

    pub(super) fn update_filter(search: &mut FileSearch) {
        if search.query.is_empty() {
            fill_initial_filter(&mut search.filtered, search.items.len());
        } else {
            search.filtered = fuzzy_filter(&search.items, &search.query);
        }

        if search.selected >= search.filtered.len() {
            search.selected = search.filtered.len().saturating_sub(1);
        }
    }

    fn fill_initial_filter(filtered: &mut Vec<(usize, u32)>, len: usize) {
        filtered.clear();
        filtered.extend((0..len).map(|index| (index, 0)));
    }

    fn fuzzy_filter(items: &[SearchItem], query: &str) -> Vec<(usize, u32)> {
        let mut matcher = Matcher::default();
        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
        let mut buf = Vec::new();
        let mut filtered = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            buf.clear();
            let utf32 = Utf32Str::new(&item.relative, &mut buf);
            if let Some(score) = pattern.score(utf32, &mut matcher) {
                filtered.push((index, score));
            }
        }
        filtered.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
        filtered
    }
}
mod input {
    use super::{filter, FileSearch, SearchAction};
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    pub(super) fn handle_key(search: &mut FileSearch, key: KeyEvent) -> SearchAction {
        if key.kind != KeyEventKind::Press {
            return SearchAction::None;
        }

        match key.code {
            KeyCode::Esc => SearchAction::Cancel,
            KeyCode::Enter => search
                .selected_path()
                .map(|path| SearchAction::Submit(path.to_path_buf()))
                .unwrap_or(SearchAction::Cancel),
            KeyCode::Up => {
                if search.selected > 0 {
                    search.selected -= 1;
                }
                SearchAction::None
            }
            KeyCode::Down => {
                if search.selected + 1 < search.filtered.len() {
                    search.selected += 1;
                }
                SearchAction::None
            }
            KeyCode::Delete if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if !search.query.is_empty() {
                    search.query.clear();
                    filter::update_filter(search);
                }
                SearchAction::None
            }
            KeyCode::Backspace => {
                if search.query.pop().is_some() {
                    filter::update_filter(search);
                }
                SearchAction::None
            }
            KeyCode::Char(c) => {
                search.query.push(c);
                filter::update_filter(search);
                SearchAction::None
            }
            _ => SearchAction::None,
        }
    }
}

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
