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
}
