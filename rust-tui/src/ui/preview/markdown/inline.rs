use super::style::inline_code_style;
use crate::text_match::contains_ignore_case;
use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

pub(crate) fn tokenize_inline_code(
    text: &str,
    base_style: Style,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find('`') {
        let before = &rest[..start];
        if !before.is_empty() {
            spans.push(Span::styled(before.to_string(), base_style));
        }

        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            spans.push(Span::styled(rest.to_string(), base_style));
            return spans;
        };

        let code = &after_start[..end];
        if !code.is_empty() {
            spans.push(Span::styled(code.to_string(), inline_code_style(theme)));
        }
        rest = &after_start[end + 1..];
    }

    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_string(), base_style));
    }

    spans
}

pub(crate) fn retokenize_inline_code(
    spans: Vec<Span<'static>>,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    for span in spans {
        let content = span.content.into_owned();
        if content.contains('`') {
            out.extend(tokenize_inline_code(&content, span.style, theme));
        } else {
            out.push(Span::styled(content, span.style));
        }
    }
    out
}

pub(crate) fn format_line(line: &str, theme: &Theme) -> Vec<Span<'static>> {
    let stripped = line.trim();

    let user_markers = ["$", "#", "❯", ">", "%"];
    for marker in &user_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            let mut spans = vec![Span::styled(
                (*marker).to_string(),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(tokenize_inline_code(
                &format!(" {}", content),
                Style::default().fg(theme.success),
                theme,
            ));
            return spans;
        }
    }

    let ai_markers = ["●", "•", "💫", "🤖", "🟣", "🔵", "🟢", "⚡"];
    for marker in &ai_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            let mut spans = vec![Span::styled(
                (*marker).to_string(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(tokenize_inline_code(
                &format!(" {}", content),
                Style::default().fg(theme.accent),
                theme,
            ));
            return spans;
        }
    }

    if contains_ignore_case(stripped, "error") || contains_ignore_case(stripped, "failed") {
        return tokenize_inline_code(line, Style::default().fg(theme.error), theme);
    }

    if contains_ignore_case(stripped, "success")
        || contains_ignore_case(stripped, "done")
        || stripped.contains("✓")
    {
        return tokenize_inline_code(line, Style::default().fg(theme.success), theme);
    }

    tokenize_inline_code(line, Style::default(), theme)
}

#[cfg(test)]
#[path = "inline_tests.rs"]
mod tests;
