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
