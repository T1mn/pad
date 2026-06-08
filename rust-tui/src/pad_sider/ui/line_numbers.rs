use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

pub fn add_line_numbers(mut text: Text<'static>) -> Text<'static> {
    let width = text.lines.len().max(1).to_string().len();
    text.lines = text
        .lines
        .into_iter()
        .enumerate()
        .map(|(index, mut line)| {
            let prefix = format!("{:>width$} │ ", index + 1, width = width);
            line.spans.insert(
                0,
                Span::styled(prefix, Style::default().fg(Color::DarkGray)),
            );
            line
        })
        .collect();
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
