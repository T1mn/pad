use super::visible_plain_line_window;
use ratatui::text::Line;

#[test]
fn visible_plain_line_window_keeps_only_rows_needed_for_viewport() {
    let lines = (0..100)
        .map(|idx| Line::from(format!("line {idx}")))
        .collect::<Vec<_>>();

    let (range, local_scroll) = visible_plain_line_window(&lines, 80, 50, 10);

    assert_eq!(range, 50..60);
    assert_eq!(local_scroll, 0);
}

#[test]
fn visible_plain_line_window_starts_inside_wrapped_line() {
    let lines = vec![
        Line::from("abcdef"),
        Line::from("gh"),
        Line::from("ij"),
        Line::from("kl"),
    ];

    let (range, local_scroll) = visible_plain_line_window(&lines, 2, 2, 2);

    assert_eq!(range, 0..2);
    assert_eq!(local_scroll, 2);
}
