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
mod tests {
    use super::*;

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
