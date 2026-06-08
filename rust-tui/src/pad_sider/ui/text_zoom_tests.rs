use super::{apply_text_zoom, is_blank};
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
