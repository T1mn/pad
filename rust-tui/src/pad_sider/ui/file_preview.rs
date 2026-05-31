use super::super::app::{App, Focus};
use super::super::preview::PreviewKind;
use super::diff::render_diff_patch;
use super::line_numbers::{add_line_numbers, text_lines};
use super::markdown::render_markdown;
use super::render::focus_block;
use super::render_window::visible_line_window;
use super::text_zoom::apply_text_zoom;
use ratatui::{
    layout::Rect,
    text::Text,
    widgets::{Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, Instant};

pub(super) fn draw_file_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(" preview {} ", app.file_preview.title);
    let block = focus_block(&title, app.focus == Focus::Preview);
    let inner = block.inner(area);
    ensure_rendered_file_preview(app, inner.width);

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

fn ensure_rendered_file_preview(app: &mut App, width: u16) {
    if app.rendered_file_preview_matches(width) {
        return;
    }

    let started_at = Instant::now();
    let text = match app.file_preview.kind {
        PreviewKind::Markdown => render_markdown(&app.file_preview.content),
        PreviewKind::Diff => render_diff_patch(&app.file_preview.content, width),
        _ => text_lines(&app.file_preview.content),
    };
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

#[cfg(test)]
mod tests {
    use super::draw_file_preview;
    use crate::pad_sider::{
        app::App,
        preview::{FilePreview, PreviewKind},
    };
    use ratatui::{backend::TestBackend, layout::Rect, Terminal};
    use std::time::Instant;

    #[test]
    #[ignore]
    fn bench_cached_diff_scroll_render_from_env() {
        let iterations = std::env::var("PAD_SIDER_BENCH_ITERATIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(20);
        let mut app = App::new(std::env::temp_dir(), None);
        app.set_file_preview(FilePreview::new(
            "large.diff".into(),
            large_patch(8, 180),
            PreviewKind::Diff,
        ));
        app.focus_preview();
        let mut terminal = Terminal::new(TestBackend::new(140, 42)).unwrap();

        let first_started = Instant::now();
        draw_once(&mut terminal, &mut app);
        let first_ms = first_started.elapsed().as_secs_f64() * 1000.0;

        let mut cached_ms = Vec::with_capacity(iterations);
        for idx in 0..iterations {
            app.file_preview.scroll = idx as u16;
            let started = Instant::now();
            draw_once(&mut terminal, &mut app);
            cached_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        }

        let avg_ms = cached_ms.iter().sum::<f64>() / cached_ms.len() as f64;
        let min_ms = cached_ms.iter().copied().fold(f64::INFINITY, f64::min);
        let max_ms = cached_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        println!(
            "bench.pad_sider_diff_render first_ms={first_ms:.3} cached_avg_ms={avg_ms:.3} cached_min_ms={min_ms:.3} cached_max_ms={max_ms:.3} iterations={iterations} rendered_lines={}",
            app.rendered_file_preview
                .as_ref()
                .map(|cache| cache.lines.len())
                .unwrap_or_default()
        );
    }

    fn draw_once(terminal: &mut Terminal<TestBackend>, app: &mut App) {
        terminal
            .draw(|frame| draw_file_preview(frame, app, Rect::new(0, 0, 140, 42)))
            .unwrap();
    }

    fn large_patch(files: usize, rows_per_file: usize) -> String {
        let mut out = String::from("Codex turn diff\n\n");
        for file in 0..files {
            out.push_str(&format!(
                "diff --git a/src/file_{file}.rs b/src/file_{file}.rs\n"
            ));
            out.push_str("index 111..222 100644\n");
            out.push_str("@@ -1,180 +1,180 @@\n");
            for row in 0..rows_per_file {
                out.push_str(&format!(" context line {row}\n"));
                out.push_str(&format!("-old value {row}\n"));
                out.push_str(&format!("+new value {row}\n"));
            }
        }
        out
    }
}
