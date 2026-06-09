mod cache;

#[cfg(test)]
#[path = "file_preview_tests.rs"]
mod file_preview_tests;

use super::super::app::{App, Focus};
use super::super::preview::PreviewKind;
use super::diff::render_diff_patch;
use super::line_numbers::{add_line_numbers, text_lines};
use super::markdown::render_markdown;
use super::render::focus_block;
use super::render_window::visible_line_window;
use super::syntax;
use super::text_zoom::apply_text_zoom;
use ratatui::{
    layout::Rect,
    text::Text,
    widgets::{Paragraph, Wrap},
    Frame,
};

pub(super) fn draw_file_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = focus_block(
        preview_title(&app.file_preview.title, app.file_preview.kind),
        app.focus == Focus::Preview,
    );
    let inner = block.inner(area);
    cache::ensure_rendered_file_preview(app, inner.width);

    let (lines, local_scroll) = app
        .rendered_file_preview
        .as_ref()
        .map(|cache| {
            let (range, local_scroll) = visible_line_window(
                &cache.lines,
                inner.width as usize,
                app.file_preview.scroll,
                inner.height as usize,
            );
            (cache.lines[range].to_vec(), local_scroll)
        })
        .unwrap_or_default();

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((local_scroll, 0));
    frame.render_widget(paragraph, area);
}

pub(super) fn with_preview_display_options(
    text: Text<'static>,
    show_line_numbers: bool,
    text_zoom: i8,
) -> Text<'static> {
    let text = if show_line_numbers {
        add_line_numbers(text)
    } else {
        text
    };
    apply_text_zoom(text, text_zoom)
}

pub(super) fn render_preview_text(
    title: &str,
    content: &str,
    kind: PreviewKind,
    width: u16,
) -> Text<'static> {
    match kind {
        PreviewKind::Markdown => render_markdown(content),
        PreviewKind::Diff => render_diff_patch(content, width),
        PreviewKind::Text => syntax::render_code(title, content),
        PreviewKind::Directory | PreviewKind::Missing => text_lines(content),
    }
}

pub(super) fn preview_title(title: &str, kind: PreviewKind) -> String {
    match kind {
        PreviewKind::Text => syntax::language_label_for_title(title)
            .map(|language| format!(" preview {title} · {language} · VS Code Dark+ "))
            .unwrap_or_else(|| format!(" preview {title} ")),
        _ => format!(" preview {title} "),
    }
}
