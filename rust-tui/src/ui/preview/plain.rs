mod cache;
mod scroll;
mod window;

use super::markdown::format_line;
use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};
use window::visible_plain_line_window;

pub(crate) use cache::ensure_plain_preview_cache;
pub(crate) use scroll::resolve_preview_scroll_from_cache;

pub(crate) fn draw_plain_preview(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    with_block: bool,
    block: &Block,
    theme: &Theme,
) {
    let viewport = if with_block { block.inner(area) } else { area };
    ensure_plain_preview_cache(app, viewport.width);

    let scroll = resolve_preview_scroll_from_cache(app, viewport);
    let (lines, local_scroll) = visible_plain_lines(app, viewport, scroll);

    let mut paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: false })
        .scroll((local_scroll, 0));
    if with_block {
        paragraph = paragraph.block(block.clone());
    }

    f.render_widget(paragraph, area);
}

fn visible_plain_lines(app: &App, viewport: Rect, scroll: u16) -> (Vec<Line<'static>>, u16) {
    app.preview
        .plain_cache
        .as_ref()
        .map(|cache| {
            let (range, local_scroll) = visible_plain_line_window(
                &cache.lines,
                viewport.width as usize,
                scroll,
                viewport.height as usize,
            );
            (cache.lines[range].to_vec(), local_scroll)
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "plain_tests.rs"]
mod tests;
