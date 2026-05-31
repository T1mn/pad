mod window;

use super::common::display_width;
use super::markdown::format_line;
use crate::app::{App, PreviewPlainCache};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, Instant};
use window::{line_wrapped_rows, visible_plain_line_window};

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
    let (lines, local_scroll) = app
        .preview
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
        .unwrap_or_default();

    let mut paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: false })
        .scroll((local_scroll, 0));
    if with_block {
        paragraph = paragraph.block(block.clone());
    }

    f.render_widget(paragraph, area);
}

pub(crate) fn ensure_plain_preview_cache(app: &mut App, width: u16) {
    let target_key = app.preview.pane_id.as_deref().unwrap_or_default();
    let theme_name = app.theme.name;
    let content = app.preview.content.as_str();
    let cache_hit = app.preview.plain_cache.as_ref().is_some_and(|cache| {
        cache.target_key == target_key
            && cache.width == width
            && cache.theme_name == theme_name
            && cache.content == content
    });
    if cache_hit {
        return;
    }

    let started_at = Instant::now();
    let lines: Vec<Line<'static>> = content
        .lines()
        .map(|line| Line::from(format_line(line, &app.theme)))
        .collect();
    let wrapped_rows = wrapped_row_count_for_lines(&lines, width as usize);
    let elapsed = started_at.elapsed();
    if elapsed >= Duration::from_millis(8) {
        crate::log_debug!(
            "preview.plain: render_slow target={} width={} lines={} elapsed_ms={}",
            target_key,
            width,
            lines.len(),
            elapsed.as_millis()
        );
    }
    app.preview.plain_cache = Some(PreviewPlainCache {
        target_key: target_key.to_string(),
        width,
        theme_name: theme_name.to_string(),
        content: content.to_string(),
        lines,
        wrapped_rows,
    });
}

pub(crate) fn resolve_preview_scroll_from_cache(app: &mut App, viewport: Rect) -> u16 {
    let max_scroll = app
        .preview
        .plain_cache
        .as_ref()
        .filter(|cache| cache.width == viewport.width)
        .map(|cache| {
            cache
                .wrapped_rows
                .saturating_sub(viewport.height as usize)
                .min(u16::MAX as usize) as u16
        })
        .unwrap_or_else(|| {
            precise_preview_max_scroll(&app.preview.content, viewport.width, viewport.height)
        });
    let scroll = if app.preview.follow_bottom {
        max_scroll
    } else {
        app.preview.scroll.min(max_scroll)
    };
    app.preview.scroll = scroll;
    scroll
}

pub(crate) fn precise_preview_max_scroll(
    content: &str,
    viewport_width: u16,
    viewport_height: u16,
) -> u16 {
    if viewport_width == 0 || viewport_height == 0 {
        return 0;
    }

    let wrapped_rows = wrapped_row_count(content, viewport_width as usize);
    let max_scroll = wrapped_rows.saturating_sub(viewport_height as usize);
    max_scroll.min(u16::MAX as usize) as u16
}

pub(crate) fn wrapped_row_count(content: &str, viewport_width: usize) -> usize {
    if viewport_width == 0 {
        return 0;
    }

    let mut total = 0usize;
    for line in content.lines() {
        let width = display_width(line);
        let rows = if width == 0 {
            1
        } else {
            width.div_ceil(viewport_width)
        };
        total += rows.max(1);
    }

    if total == 0 {
        1
    } else {
        total
    }
}

pub(crate) fn wrapped_row_count_for_lines(lines: &[Line<'_>], viewport_width: usize) -> usize {
    if viewport_width == 0 {
        return 0;
    }

    let mut total = 0usize;
    for line in lines {
        total += line_wrapped_rows(line, viewport_width);
    }

    if total == 0 {
        1
    } else {
        total
    }
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
        assert_eq!(initial_cache.content, repeated_cache.content);
        assert_eq!(initial_cache.wrapped_rows, repeated_cache.wrapped_rows);
        assert_eq!(initial_cache.lines.len(), repeated_cache.lines.len());
    }
}
