mod render;
mod scan {
    /// Scan directories up to max_depth (public for use by App)
    pub fn scan_directories(base: &str, max_depth: usize) -> Vec<String> {
        let mut results = vec![base.to_string()];

        if max_depth == 0 {
            return results;
        }

        let base_path = std::path::Path::new(base);
        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let path_str = path.to_string_lossy().to_string();

                    // Skip hidden directories
                    if let Some(name) = path.file_name() {
                        if name.to_string_lossy().starts_with('.') {
                            continue;
                        }
                    }

                    // Recursively scan (limit depth)
                    if max_depth > 1 {
                        let sub_dirs = scan_directories(&path_str, max_depth - 1);
                        results.extend(sub_dirs.into_iter().skip(1)); // Skip duplicate base
                    }
                    results.push(path_str);
                }
            }
        }

        // Sort and remove duplicates
        results.sort_unstable();
        results.dedup();
        results
    }
}
mod search {
    use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
    use nucleo_matcher::{Matcher, Utf32Str};

    pub(super) fn filter_items(items: &[String], query: &str) -> Vec<(String, u32)> {
        if query.is_empty() {
            let mut results = Vec::with_capacity(items.len());
            fill_unfiltered(items, &mut results);
            return results;
        }

        let mut matcher = Matcher::default();
        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
        let mut buf = Vec::new();
        let mut results = Vec::with_capacity(items.len());
        for item in items {
            buf.clear();
            let utf32_str = Utf32Str::new(item, &mut buf);
            if let Some(score) = pattern.score(utf32_str, &mut matcher) {
                results.push((item.clone(), score));
            }
        }

        results.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        results
    }

    pub(super) fn fill_unfiltered(items: &[String], filtered: &mut Vec<(String, u32)>) {
        filtered.clear();
        filtered.extend(items.iter().map(|item| (item.clone(), 0)));
    }
}

pub use scan::scan_directories;

/// Fuzzy finder state
pub struct FuzzyPicker {
    /// All items to search
    items: Vec<String>,
    /// Filtered items with scores
    filtered: Vec<(String, u32)>,
    /// Current search query
    query: String,
    /// Selected index in filtered list
    selected: usize,
    /// Whether the picker is active
    active: bool,
}

impl FuzzyPicker {
    pub fn new(items: Vec<String>) -> Self {
        let mut filtered = Vec::with_capacity(items.len());
        search::fill_unfiltered(&items, &mut filtered);
        Self {
            items,
            filtered,
            query: String::new(),
            selected: 0,
            active: true,
        }
    }

    /// Update filter based on current query
    fn update_filter(&mut self) {
        if self.query.is_empty() {
            search::fill_unfiltered(&self.items, &mut self.filtered);
        } else {
            self.filtered = search::filter_items(&self.items, &self.query);
        }

        // Reset selection if out of bounds
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn clear_query(&mut self) {
        self.query.clear();
        self.update_filter();
    }

    /// Handle keyboard input. Returns:
    /// - None: no action (continue)
    /// - Some(None): cancelled (Esc)
    /// - Some(Some(path)): selected a path
    pub fn handle_input(&mut self, key: crossterm::event::KeyEvent) -> Option<Option<String>> {
        use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

        if key.kind != KeyEventKind::Press {
            return None;
        }

        match key.code {
            KeyCode::Delete if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.clear_query();
                None
            }
            KeyCode::Esc => {
                self.active = false;
                Some(None) // Cancelled
            }
            KeyCode::Enter => {
                self.active = false;
                if let Some((item, _)) = self.filtered.get(self.selected) {
                    Some(Some(item.clone()))
                } else {
                    Some(None)
                }
            }
            // Only arrow keys for navigation — j/k go to the Char(c) catch-all so users can type them
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                None
            }
            KeyCode::Down => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
                None
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.update_filter();
                None
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.update_filter();
                None
            }
            _ => None,
        }
    }

    pub fn draw(&self, f: &mut ratatui::Frame) {
        render::draw_picker(self, f);
    }
}

#[cfg(test)]
#[path = "fuzzy_tests.rs"]
mod tests;
