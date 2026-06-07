use super::super::App;

impl App {
    pub fn scroll_preview_by(&mut self, delta: i32) {
        if self.preview.uses_list_scroll() {
            self.preview.follow_selection = false;
            if delta >= 0 {
                self.preview.list_scroll = self.preview.list_scroll.saturating_add(delta as u16);
            } else {
                self.preview.list_scroll = self.preview.list_scroll.saturating_sub((-delta) as u16);
            }
        } else if self.preview.uses_detail_scroll() {
            if delta >= 0 {
                self.preview.detail_scroll =
                    self.preview.detail_scroll.saturating_add(delta as u16);
            } else {
                self.preview.detail_scroll =
                    self.preview.detail_scroll.saturating_sub((-delta) as u16);
            }
        } else {
            self.preview.follow_bottom = false;
            if delta >= 0 {
                self.preview.scroll = self.preview.scroll.saturating_add(delta as u16);
            } else {
                self.preview.scroll = self.preview.scroll.saturating_sub((-delta) as u16);
            }
        }
        self.dirty = true;
    }

    pub fn scroll_preview_to_top(&mut self) {
        if self.preview.uses_list_scroll() {
            self.preview.list_scroll = 0;
            self.preview.follow_selection = false;
        } else if self.preview.uses_detail_scroll() {
            self.preview.detail_scroll = 0;
        } else {
            self.preview.scroll = 0;
            self.preview.follow_bottom = false;
        }
        self.dirty = true;
    }

    pub fn scroll_preview_to_bottom(&mut self) {
        if self.preview.uses_list_scroll() {
            self.preview.list_scroll = u16::MAX;
            self.preview.follow_selection = false;
        } else if self.preview.uses_detail_scroll() {
            self.preview.detail_scroll = u16::MAX;
        } else {
            self.preview.follow_bottom = true;
        }
        self.dirty = true;
    }
}
