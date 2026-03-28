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

pub(crate) fn draw_plain_preview(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    with_block: bool,
    block: &Block,
    theme: &Theme,
) {
    let viewport = if with_block { block.inner(area) } else { area };
    let target_key = app.preview_pane_id.clone().unwrap_or_default();
    let theme_name = theme.name.to_string();
    let content = app.preview_content.clone();
    let cache_hit = app.preview_plain_cache.as_ref().is_some_and(|cache| {
        cache.target_key == target_key
            && cache.width == viewport.width
            && cache.theme_name == theme_name
            && cache.content == content
    });
    if !cache_hit {
        let started_at = Instant::now();
        let lines: Vec<Line<'static>> = content
            .lines()
            .map(|line| Line::from(format_line(line, theme)))
            .collect();
        let wrapped_rows = wrapped_row_count_for_lines(&lines, viewport.width as usize);
        let elapsed = started_at.elapsed();
        if elapsed >= Duration::from_millis(8) {
            crate::log_debug!(
                "preview.plain: render_slow target={} width={} lines={} elapsed_ms={}",
                target_key,
                viewport.width,
                lines.len(),
                elapsed.as_millis()
            );
        }
        app.preview_plain_cache = Some(PreviewPlainCache {
            target_key,
            width: viewport.width,
            theme_name,
            content,
            lines,
            wrapped_rows,
        });
    }

    let scroll = resolve_preview_scroll_from_cache(app, viewport);
    let lines = app
        .preview_plain_cache
        .as_ref()
        .map(|cache| cache.lines.clone())
        .unwrap_or_default();

    let mut paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    if with_block {
        paragraph = paragraph.block(block.clone());
    }

    f.render_widget(paragraph, area);
}

pub(crate) fn resolve_preview_scroll_from_cache(app: &mut App, viewport: Rect) -> u16 {
    let max_scroll = app
        .preview_plain_cache
        .as_ref()
        .filter(|cache| cache.width == viewport.width)
        .map(|cache| {
            cache
                .wrapped_rows
                .saturating_sub(viewport.height as usize)
                .min(u16::MAX as usize) as u16
        })
        .unwrap_or_else(|| {
            precise_preview_max_scroll(&app.preview_content, viewport.width, viewport.height)
        });
    let scroll = if app.preview_follow_bottom {
        max_scroll
    } else {
        app.preview_scroll.min(max_scroll)
    };
    app.preview_scroll = scroll;
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
        let width = line
            .spans
            .iter()
            .map(|span| display_width(span.content.as_ref()))
            .sum::<usize>();
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
