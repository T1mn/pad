use super::app::App;
use super::preview::{FilePreview, RenderedFilePreview};

impl App {
    pub(crate) fn set_file_preview(&mut self, preview: FilePreview) {
        if file_preview_body_changed(&self.file_preview, &preview) {
            self.file_preview_revision = self.file_preview_revision.wrapping_add(1);
            self.rendered_file_preview = None;
        }
        self.file_preview = preview;
    }

    pub(crate) fn rendered_file_preview_matches(&self, width: u16) -> bool {
        self.rendered_file_preview.as_ref().is_some_and(|cache| {
            cache.revision == self.file_preview_revision
                && cache.width == width
                && cache.show_line_numbers == self.show_line_numbers
                && cache.text_zoom == self.text_zoom
        })
    }

    pub(crate) fn store_rendered_file_preview(
        &mut self,
        width: u16,
        lines: Vec<ratatui::text::Line<'static>>,
    ) {
        self.rendered_file_preview = Some(RenderedFilePreview {
            revision: self.file_preview_revision,
            width,
            show_line_numbers: self.show_line_numbers,
            text_zoom: self.text_zoom,
            lines,
        });
    }
}

fn file_preview_body_changed(previous: &FilePreview, next: &FilePreview) -> bool {
    previous.title != next.title || previous.kind != next.kind || previous.content != next.content
}

#[cfg(test)]
#[path = "preview_render_cache_tests.rs"]
mod tests;
