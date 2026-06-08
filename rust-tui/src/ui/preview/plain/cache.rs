use super::format_line;
use super::scroll::wrapped_row_count_for_lines;
use crate::app::{App, PreviewPlainCache};
use ratatui::text::Line;
use std::time::{Duration, Instant};

pub(crate) fn ensure_plain_preview_cache(app: &mut App, width: u16) {
    let target_key = app.preview.pane_id.as_deref().unwrap_or_default();
    let theme_name = app.theme.name;
    let content = app.preview.content.as_str();
    let content_revision = app.preview.content_revision;
    let cache_hit = app.preview.plain_cache.as_ref().is_some_and(|cache| {
        cache.target_key == target_key
            && cache.width == width
            && cache.theme_name == theme_name
            && cache.content_revision == content_revision
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
        content_revision,
        lines,
        wrapped_rows,
    });
}
