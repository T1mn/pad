use super::{display_width, visible_line_window};
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
