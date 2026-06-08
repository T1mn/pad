use super::super::common::display_width;
use super::window::line_wrapped_rows;
use crate::app::App;
use ratatui::{layout::Rect, text::Line};

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

fn wrapped_row_count(content: &str, viewport_width: usize) -> usize {
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

pub(super) fn wrapped_row_count_for_lines(lines: &[Line<'_>], viewport_width: usize) -> usize {
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
