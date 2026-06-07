use super::fs::relative_path_label;
use super::tree::scan_files;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
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
        let filtered = (0..items.len()).map(|index| (index, 0)).collect();
        Self {
            items,
            filtered,
            query: String::new(),
            selected: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SearchAction {
        if key.kind != KeyEventKind::Press {
            return SearchAction::None;
        }

        match key.code {
            KeyCode::Esc => SearchAction::Cancel,
            KeyCode::Enter => self
                .selected_path()
                .map(|path| SearchAction::Submit(path.to_path_buf()))
                .unwrap_or(SearchAction::Cancel),
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                SearchAction::None
            }
            KeyCode::Down => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
                SearchAction::None
            }
            KeyCode::Delete if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.query.clear();
                self.update_filter();
                SearchAction::None
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.update_filter();
                SearchAction::None
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.update_filter();
                SearchAction::None
            }
            _ => SearchAction::None,
        }
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

    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.items.len()).map(|index| (index, 0)).collect();
        } else {
            let mut matcher = Matcher::default();
            let pattern = Pattern::parse(&self.query, CaseMatching::Smart, Normalization::Smart);
            let mut buf = Vec::new();
            let mut filtered = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    buf.clear();
                    let utf32 = Utf32Str::new(&item.relative, &mut buf);
                    pattern
                        .score(utf32, &mut matcher)
                        .map(|score| (index, score))
                })
                .collect::<Vec<_>>();
            filtered.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
            self.filtered = filtered;
        }

        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FileSearch;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_search_dir() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pad-sider-search-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("alpha.rs"), "fn alpha() {}").unwrap();
        fs::write(dir.join("beta.rs"), "fn beta() {}").unwrap();
        dir
    }

    #[test]
    fn shift_delete_clears_query() {
        let dir = temp_search_dir();
        let mut search = FileSearch::new(&dir);

        search.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert_eq!(search.query, "z");
        assert!(search.filtered.is_empty());

        search.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
        assert!(search.query.is_empty());
        assert_eq!(search.filtered.len(), 2);

        fs::remove_dir_all(dir).unwrap();
    }
}
