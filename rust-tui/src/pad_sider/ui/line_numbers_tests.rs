use super::{add_line_numbers, text_lines};
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
