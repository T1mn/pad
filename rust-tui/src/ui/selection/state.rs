#[derive(Clone, Debug, Default)]
pub struct SelectionState {
    pub selected: usize,
    pub scroll: u16,
    pub query: String,
    pub searching: bool,
}

impl SelectionState {
    pub fn clamp_selected(&mut self, len: usize) {
        self.selected = self.selected.min(len.saturating_sub(1));
        if len == 0 {
            self.selected = 0;
            self.scroll = 0;
        }
    }

    pub fn filtered_indices<T>(
        &self,
        items: &[T],
        matches_query: impl Fn(&T, &str) -> bool,
    ) -> Vec<usize> {
        items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| matches_query(item, &self.query).then_some(idx))
            .collect()
    }

    pub fn selected_filtered_index<T>(
        &self,
        items: &[T],
        matches_query: impl Fn(&T, &str) -> bool,
    ) -> Option<usize> {
        let filtered = self.filtered_indices(items, matches_query);
        if filtered.is_empty() {
            None
        } else {
            Some(self.selected.min(filtered.len().saturating_sub(1)))
        }
    }
}
