use super::super::app::App;
use super::super::fs::{is_markdown_file, read_markdown_file};
use super::super::preview::MarkdownPreview;

impl App {
    pub fn open_preview(&mut self) {
        let Some(path) = self.selected_path().cloned() else {
            return;
        };
        if !is_markdown_file(&path) {
            return;
        }
        self.preview = Some(MarkdownPreview {
            content: read_markdown_file(&path),
            path,
            scroll: 0,
        });
    }

    pub fn close_preview(&mut self) {
        self.preview = None;
    }

    pub fn preview_down(&mut self) {
        if let Some(preview) = self.preview.as_mut() {
            preview.scroll = preview.scroll.saturating_add(1);
        }
    }

    pub fn preview_up(&mut self) {
        if let Some(preview) = self.preview.as_mut() {
            preview.scroll = preview.scroll.saturating_sub(1);
        }
    }

    pub fn reset_preview(&mut self) {
        if let Some(preview) = self.preview.as_mut() {
            preview.scroll = 0;
        }
    }

    pub fn preview_bottom(&mut self) {
        if let Some(preview) = self.preview.as_mut() {
            preview.scroll = u16::MAX;
        }
    }

    pub fn file_preview_down(&mut self) {
        self.file_preview_scroll_down(8);
    }

    pub fn file_preview_up(&mut self) {
        self.file_preview_scroll_up(8);
    }

    pub fn file_preview_scroll_down(&mut self, amount: u16) {
        self.file_preview.scroll = self.file_preview.scroll.saturating_add(amount);
    }

    pub fn file_preview_scroll_up(&mut self, amount: u16) {
        self.file_preview.scroll = self.file_preview.scroll.saturating_sub(amount);
    }
}
