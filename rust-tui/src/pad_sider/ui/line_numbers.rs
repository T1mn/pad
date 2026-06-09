use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use std::fmt::Write as _;

pub fn add_line_numbers(mut text: Text<'static>) -> Text<'static> {
    let width = text.lines.len().max(1).to_string().len();
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
