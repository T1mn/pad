use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use std::fmt::Write as _;

pub fn add_line_numbers(mut text: Text<'static>) -> Text<'static> {
    let width = decimal_width(text.lines.len().max(1));
    for (index, line) in text.lines.iter_mut().enumerate() {
        let mut prefix = String::with_capacity(width + " │ ".len());
        let _ = write!(prefix, "{:>width$} │ ", index + 1, width = width);
        line.spans.insert(
            0,
            Span::styled(prefix, Style::default().fg(Color::DarkGray)),
        );
    }
    text
}

fn decimal_width(mut value: usize) -> usize {
    let mut width = 1;
    while value >= 10 {
        value /= 10;
        width += 1;
    }
    width
}

pub fn text_lines(content: &str) -> Text<'static> {
    let lines = content
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect::<Vec<_>>();
    Text::from(lines)
}

#[cfg(test)]
#[path = "line_numbers_tests.rs"]
mod tests;
