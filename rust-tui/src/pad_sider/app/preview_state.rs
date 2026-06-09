use super::{App, NavMode};
use crate::pad_sider::preview::FilePreview;

impl App {
    pub(crate) fn refresh_file_preview(&mut self) {
        let path = match self.nav_mode {
            NavMode::Tree => self.tree.get(self.selected).map(|row| &row.path),
            NavMode::IndexMap => self
                .index_rows
                .get(self.index_selected)
                .map(|row| &row.path),
            NavMode::CodexRuns => {
                self.refresh_codex_diff_preview();
                return;
            }
        };
        self.codex_diff_preview_key = None;
        let previous_scroll = self.file_preview.scroll;
        let mut preview = path
            .map(|path| self.file_preview_cache.preview_for(&self.cwd, path))
            .unwrap_or_else(FilePreview::empty);
        if preview.title == self.file_preview.title {
            preview.scroll = previous_scroll;
        }
        self.set_file_preview(preview);
    }

    pub(crate) fn refresh_preview(&mut self) -> bool {
        let Some(preview) = self.preview.as_mut() else {
            return false;
        };
        if preview.path.is_file() {
            let scroll = preview.preview.scroll;
            let mut refreshed = self
                .file_preview_cache
                .preview_for(&self.cwd, &preview.path);
            refreshed.scroll = scroll;
            if preview.preview == refreshed {
                return false;
            }
            preview.preview = refreshed;
            true
        } else {
            self.preview = None;
            true
        }
    }
}
