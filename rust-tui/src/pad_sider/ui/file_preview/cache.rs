use super::{render_preview_text, with_preview_display_options};
use crate::pad_sider::app::App;
use crate::pad_sider::preview::PreviewKind;
use std::time::{Duration, Instant};

pub(super) fn ensure_rendered_file_preview(app: &mut App, width: u16) {
    if app.rendered_file_preview_matches(width) {
        return;
    }

    let started_at = Instant::now();
    let text = render_preview_text(
        &app.file_preview.title,
        &app.file_preview.content,
        app.file_preview.kind,
        width,
    );
    let text = with_preview_display_options(text, app.show_line_numbers, app.text_zoom);
    let line_count = text.lines.len();
    app.store_rendered_file_preview(width, text.lines);

    let elapsed = started_at.elapsed();
    if elapsed >= Duration::from_millis(8) {
        crate::log_debug!(
            "pad_sider.render_cache: rebuild kind={} width={} lines={} bytes={} elapsed_ms={}",
            preview_kind_label(app.file_preview.kind),
            width,
            line_count,
            app.file_preview.content.len(),
            elapsed.as_millis()
        );
    }
}

fn preview_kind_label(kind: PreviewKind) -> &'static str {
    match kind {
        PreviewKind::Markdown => "markdown",
        PreviewKind::Text => "text",
        PreviewKind::Diff => "diff",
        PreviewKind::Directory => "directory",
        PreviewKind::Missing => "missing",
    }
}
