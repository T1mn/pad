use crate::theme::Theme;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(super) fn format_plain_file_preview(content: &str, theme: &Theme) -> Vec<Line<'static>> {
    content
        .lines()
        .map(|line| Line::from(format_file_preview_line(line, theme)))
        .collect()
}

fn format_file_preview_line(line: &str, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let trimmed = line.trim_start();
    let indent = line.chars().count() - trimmed.chars().count();

    if indent > 0 {
        spans.push(Span::raw(" ".repeat(indent)));
    }

    let stripped = trimmed;

    if let Some(idx) = stripped.find("//") {
        spans.push(Span::raw(stripped[..idx].to_string()));
        spans.push(Span::styled(
            stripped[idx..].to_string(),
            Style::default().fg(theme.comment),
        ));
        return retokenize(spans, theme);
    }
    if stripped.starts_with('#') {
        spans.push(Span::styled(
            stripped.to_string(),
            Style::default().fg(theme.success),
        ));
        return retokenize(spans, theme);
    }

    if stripped.contains('"') || stripped.contains('\'') {
        let mut in_string = false;
        let mut string_start = 0;

        for (i, c) in stripped.char_indices() {
            if c == '"' || c == '\'' {
                if !in_string {
                    if i > string_start {
                        spans.push(Span::raw(stripped[string_start..i].to_string()));
                    }
                    in_string = true;
                    string_start = i;
                } else {
                    spans.push(Span::styled(
                        stripped[string_start..=i].to_string(),
                        Style::default().fg(theme.string_color),
                    ));
                    in_string = false;
                    string_start = i + 1;
                }
            }
        }

        if string_start < stripped.len() {
            if in_string {
                spans.push(Span::styled(
                    stripped[string_start..].to_string(),
                    Style::default().fg(theme.string_color),
                ));
            } else {
                spans.push(Span::raw(stripped[string_start..].to_string()));
            }
        }

        if !spans.is_empty() {
            return retokenize(spans, theme);
        }
    }

    let keywords = [
        "fn", "let", "mut", "if", "else", "for", "while", "match", "struct", "enum", "impl", "pub",
        "use", "mod", "const", "return", "true", "false", "None", "Some", "Ok", "Err",
    ];
    for kw in &keywords {
        if stripped.starts_with(kw)
            && (stripped.len() == kw.len()
                || !stripped[kw.len()..].starts_with(char::is_alphanumeric))
        {
            spans.push(Span::styled(
                (*kw).to_string(),
                Style::default().fg(theme.keyword),
            ));
            if stripped.len() > kw.len() {
                spans.push(Span::raw(stripped[kw.len()..].to_string()));
            }
            return retokenize(spans, theme);
        }
    }

    if stripped
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        let end = stripped
            .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '_')
            .unwrap_or(stripped.len());
        spans.push(Span::styled(
            stripped[..end].to_string(),
            Style::default().fg(theme.number),
        ));
        if end < stripped.len() {
            spans.push(Span::raw(stripped[end..].to_string()));
        }
        return retokenize(spans, theme);
    }

    crate::ui::preview::markdown::tokenize_inline_code(line, Style::default(), theme)
}

fn retokenize(spans: Vec<Span<'static>>, theme: &Theme) -> Vec<Span<'static>> {
    crate::ui::preview::markdown::retokenize_inline_code(spans, theme)
}
