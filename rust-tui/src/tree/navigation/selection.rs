use crate::tree::FileTree;

impl FileTree {
    /// Select next entry
    pub fn next(&mut self) {
        let count = self.entries.len();
        if count == 0 {
            return;
        }
        let index = self.state.selected().unwrap_or(0);
        if index < count - 1 {
            self.state.select(Some(index + 1));
        }
    }

    /// Select previous entry
    pub fn previous(&mut self) {
        let index = self.state.selected().unwrap_or(0);
        if index > 0 {
            self.state.select(Some(index - 1));
        }
    }
}
