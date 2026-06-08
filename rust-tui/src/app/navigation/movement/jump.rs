use super::super::super::App;

impl App {
    pub fn jump_to_sidebar_index(&mut self, index: usize) -> bool {
        self.select_sidebar_index(index, true)
    }

    pub fn jump_to(&mut self, index: usize) {
        let target_sidebar_index = {
            let items = self.visible_sidebar_items_ref();
            Self::nth_visible_thread_sidebar_index(items, index)
        };
        let Some(target_sidebar_index) = target_sidebar_index else {
            return;
        };
        self.select_sidebar_index(target_sidebar_index, true);
    }
}
