mod line_numbers {
    use super::super::line_numbers::{add_line_numbers, text_lines};
    use ratatui::text::Text;

    fn first_line(text: Text<'static>) -> String {
        text.lines[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn prefixes_text_with_line_numbers() {
        assert_eq!(
            first_line(add_line_numbers(text_lines("one\ntwo"))),
            "1 │ one"
        );
    }

    #[test]
    fn aligns_multi_digit_line_numbers() {
        let input = (0..10).map(|_| "x").collect::<Vec<_>>().join("\n");
        assert_eq!(first_line(add_line_numbers(text_lines(&input))), " 1 │ x");
    }
}

mod nav_window {
    use super::super::nav_window::{relative_selection, selected_window};

    #[test]
    fn selected_window_keeps_selection_visible() {
        assert_eq!(selected_window(100, 50, 10), 45..55);
        assert_eq!(relative_selection(50, &(45..55)), Some(5));
    }

    #[test]
    fn selected_window_clamps_edges() {
        assert_eq!(selected_window(100, 1, 10), 0..10);
        assert_eq!(selected_window(100, 98, 10), 90..100);
    }

    #[test]
    fn selected_window_handles_empty_or_tiny_viewports() {
        assert_eq!(selected_window(0, 0, 10), 0..0);
        assert_eq!(selected_window(10, 0, 0), 0..0);
        assert_eq!(selected_window(3, 9, 10), 0..3);
    }
}

mod render_window {
    use super::super::render_window::{display_width, visible_line_window};
    use ratatui::text::Line;

    #[test]
    fn visible_line_window_takes_only_visible_rows() {
        let lines = (0..100)
            .map(|idx| Line::from(format!("line {idx}")))
            .collect::<Vec<_>>();

        let (range, local_scroll) = visible_line_window(&lines, 80, 50, 10);

        assert_eq!(range, 50..60);
        assert_eq!(local_scroll, 0);
    }

    #[test]
    fn visible_line_window_starts_inside_wrapped_line() {
        let lines = vec![
            Line::from("abcdef"),
            Line::from("gh"),
            Line::from("ij"),
            Line::from("kl"),
        ];

        let (range, local_scroll) = visible_line_window(&lines, 2, 2, 2);

        assert_eq!(range, 0..2);
        assert_eq!(local_scroll, 2);
    }

    #[test]
    fn display_width_uses_ascii_width() {
        assert_eq!(display_width("src/main.rs"), 11);
    }

    #[test]
    fn display_width_handles_tabs_and_wide_chars() {
        assert_eq!(display_width("\t好🙂"), 8);
    }
}

mod split {
    use super::super::split::left_column_width;

    #[test]
    fn keeps_left_column_stable_as_sider_grows() {
        assert_eq!(left_column_width(100), 34);
        assert_eq!(left_column_width(130), 41);
        assert_eq!(left_column_width(180), 46);
    }

    #[test]
    fn avoids_over_compressing_left_column_when_narrow() {
        assert_eq!(left_column_width(70), 34);
        assert_eq!(left_column_width(60), 30);
        assert_eq!(left_column_width(50), 24);
    }
}

mod text_zoom {
    use super::super::text_zoom::{apply_text_zoom, is_blank};
    use ratatui::text::{Line, Text};

    #[test]
    fn compact_removes_blank_lines() {
        let text = Text::from(vec![Line::from("one"), Line::default(), Line::from("two")]);
        assert_eq!(apply_text_zoom(text, -1).lines.len(), 2);
    }

    #[test]
    fn roomy_adds_blank_lines_between_content() {
        let text = Text::from(vec![Line::from("one"), Line::from("two")]);
        let zoomed = apply_text_zoom(text, 1);
        assert_eq!(zoomed.lines.len(), 3);
        assert!(is_blank(&zoomed.lines[1]));
    }
}

mod file_preview {
    use super::super::file_preview::draw_file_preview;
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

mod markdown {
    use super::super::markdown::render_markdown;
    use ratatui::style::{Color, Modifier};

    fn line_texts(text: ratatui::text::Text<'_>) -> Vec<String> {
        text.lines
            .into_iter()
            .map(|line| {
                line.spans
                    .into_iter()
                    .map(|span| span.content.to_string())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn renders_blockquote_and_hides_code_block_language() {
        let lines = line_texts(render_markdown("> note\n\n```rs\nfn main() {}\n```"));
        assert!(lines.iter().any(|line| line.contains("│ note")));
        assert!(!lines.iter().any(|line| line.contains("code:rs")));
        assert!(lines.iter().any(|line| line.contains("fn main() {}")));
    }

    #[test]
    fn renders_list_markers() {
        let lines = line_texts(render_markdown("- one\n- two"));
        assert!(lines.iter().any(|line| line.contains("• one")));
        assert!(lines.iter().any(|line| line.contains("• two")));
    }

    #[test]
    fn renders_without_extra_block_spacing() {
        let lines = line_texts(render_markdown("# Title\n\nbody\n\n- one\n- two"));
        assert_eq!(lines, vec!["Title", "body", "• one", "• two"]);
    }

    #[test]
    fn preserves_blank_lines_inside_code_blocks() {
        let lines = line_texts(render_markdown("```rs\nlet a = 1;\n\nlet b = 2;\n```"));
        assert!(lines.iter().any(|line| line.contains("let a = 1;")));
        assert!(lines.iter().any(|line| line.trim().is_empty()));
        assert!(lines.iter().any(|line| line.contains("let b = 2;")));
    }

    #[test]
    fn inline_code_uses_distinct_background() {
        let text = render_markdown("run `cargo test` now");
        let span = text
            .lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content == "cargo test")
            .expect("inline code span");

        assert_eq!(span.style.fg, Some(Color::Rgb(224, 175, 104)));
        assert_eq!(span.style.bg, Some(Color::Rgb(42, 43, 61)));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn code_block_uses_distinct_background() {
        let text = render_markdown("```rs\nfn main() {}\n```");
        let spans = text
            .lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .collect::<Vec<_>>();

        let code = spans
            .iter()
            .find(|span| span.content.contains("fn main"))
            .expect("code block line");

        assert_eq!(code.style.fg, Some(Color::Rgb(255, 158, 100)));
        assert_eq!(code.style.bg, Some(Color::Rgb(26, 27, 38)));
    }

    #[test]
    fn code_block_language_changes_color() {
        let bash = render_markdown("```bash\necho ok\n```");
        let python = render_markdown("```python\nprint('ok')\n```");
        let bash_span = bash.lines[0]
            .spans
            .iter()
            .find(|span| span.content.contains("echo"))
            .unwrap();
        let python_span = python.lines[0]
            .spans
            .iter()
            .find(|span| span.content.contains("print"))
            .unwrap();

        assert_eq!(bash_span.style.fg, Some(Color::Rgb(158, 206, 106)));
        assert_eq!(python_span.style.fg, Some(Color::Rgb(122, 162, 247)));
    }

    #[test]
    fn code_block_language_is_case_insensitive_without_allocating_label() {
        let text = render_markdown(
            "```PYTHON
    print('ok')
    ```",
        );
        let span = text.lines[0]
            .spans
            .iter()
            .find(|span| span.content.contains("print"))
            .unwrap();

        assert_eq!(span.style.fg, Some(Color::Rgb(122, 162, 247)));
    }
}
