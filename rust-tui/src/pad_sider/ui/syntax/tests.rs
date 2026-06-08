use super::render_code;
use super::styles::{COMMENT, FUNCTION, KEYWORD, STRING};
use ratatui::{style::Color, text::Text};

fn colors(text: Text<'static>) -> Vec<Color> {
    text.lines
        .into_iter()
        .flat_map(|line| line.spans.into_iter())
        .filter_map(|span| span.style.fg)
        .collect()
}

#[test]
fn rust_code_uses_dark_plus_keyword_and_string_colors() {
    let text = render_code("src/main.rs", "fn main() { println!(\"hi\"); }");
    let colors = colors(text);
    assert!(colors.contains(&KEYWORD));
    assert!(colors.contains(&STRING));
    assert!(colors.contains(&FUNCTION));
}

#[test]
fn python_code_uses_comment_and_keyword_colors() {
    let text = render_code("script.py", "def run(): # comment");
    let colors = colors(text);
    assert!(colors.contains(&KEYWORD));
    assert!(colors.contains(&COMMENT));
}

#[test]
fn unknown_files_fall_back_to_plain_text() {
    let text = render_code("README.unknown", "hello");
    assert_eq!(text.lines[0].spans[0].content.as_ref(), "hello");
    assert_eq!(text.lines[0].spans[0].style.fg, None);
}

#[test]
fn uppercase_extensions_are_highlighted_without_filename_lowercase_copy() {
    let text = render_code("SRC/MAIN.RS", "fn main() { println!(\"hi\"); }");
    let colors = colors(text);
    assert!(colors.contains(&KEYWORD));
    assert!(colors.contains(&FUNCTION));
}
