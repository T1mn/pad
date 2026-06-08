use super::layout::focus_block;
use crate::pad_sider::{app::App, preview::FullscreenPreview};
use ratatui::{
    layout::Margin,
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::pad_sider::ui::file_preview::{
    preview_title, render_preview_text, with_preview_display_options,
};

pub(in crate::pad_sider::ui) fn draw_preview(
    frame: &mut Frame,
    app: &App,
    preview: &FullscreenPreview,
) {
    let title = preview_title(&preview.preview.title, preview.preview.kind);
    let inner = frame.area().inner(Margin::new(1, 1));
    let text = with_preview_display_options(
        render_preview_text(
            &preview.preview.title,
            &preview.preview.content,
            preview.preview.kind,
            inner.width,
        ),
        app.show_line_numbers,
        app.text_zoom,
    );
    let paragraph = Paragraph::new(text)
        .block(focus_block(&title, true))
        .wrap(Wrap { trim: false })
        .scroll((preview.preview.scroll, 0));
    frame.render_widget(paragraph, frame.area());
}
