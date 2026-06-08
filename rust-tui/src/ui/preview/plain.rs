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
mod tests {
    use super::ensure_plain_preview_cache;
    use crate::app::App;

    #[test]
    fn ensure_plain_preview_cache_reuses_existing_cache_when_context_is_unchanged() {
        let mut app = App::new();
        app.preview.pane_id = Some("%1".into());
        app.preview.content = "plain text".into();

        ensure_plain_preview_cache(&mut app, 12);
        let initial_cache = app.preview.plain_cache.clone().expect("cache built");

        ensure_plain_preview_cache(&mut app, 12);
        let repeated_cache = app.preview.plain_cache.expect("cache reused");

        assert_eq!(initial_cache.target_key, repeated_cache.target_key);
        assert_eq!(initial_cache.width, repeated_cache.width);
        assert_eq!(initial_cache.theme_name, repeated_cache.theme_name);
        assert_eq!(initial_cache.wrapped_rows, repeated_cache.wrapped_rows);
        assert_eq!(initial_cache.lines.len(), repeated_cache.lines.len());
    }
}
