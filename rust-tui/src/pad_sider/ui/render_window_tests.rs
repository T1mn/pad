use super::visible_line_window;
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
