mod render;
mod scan;
mod search;

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
