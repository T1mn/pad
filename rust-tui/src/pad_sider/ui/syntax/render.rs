use super::language::{language_for_title, CodeLanguage};
use super::lex::{
    comment_start, consume_string, is_number, is_operator_char, is_quote, is_word_char,
    looks_like_type,
};
use super::styles::{
    comment_style, default_style, function_style, keyword_style, number_style, operator_style,
    property_style, string_style, tag_style, type_style,
};
use super::tokens::{is_builtin, is_keyword, is_type_keyword};
use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};

pub(in crate::pad_sider::ui) fn render_code(title: &str, content: &str) -> Text<'static> {
    let Some(language) = language_for_title(title) else {
        return super::super::line_numbers::text_lines(content);
    };
    Text::from(
        content
            .lines()
            .map(|line| highlight_line(language, line))
            .collect::<Vec<_>>(),
    )
}

fn highlight_line(language: CodeLanguage, line: &str) -> Line<'static> {
    if matches!(language, CodeLanguage::Html) {
        return highlight_html_line(line);
    }

    let mut spans = Vec::new();
    let comment_at = comment_start(language, line);
    let code = comment_at.map(|idx| &line[..idx]).unwrap_or(line);
    push_code_segment(language, code, &mut spans);
    if let Some(idx) = comment_at {
        spans.push(Span::styled(line[idx..].to_string(), comment_style()));
    }
    Line::from(spans)
}

fn push_code_segment(language: CodeLanguage, segment: &str, spans: &mut Vec<Span<'static>>) {
    let mut current = String::new();
    let mut chars = segment.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if is_quote(language, ch) {
            flush_word(language, &mut current, spans);
            let (literal, next_idx) = consume_string(segment, idx, ch);
            spans.push(Span::styled(literal.to_string(), string_style()));
            while let Some((peek_idx, _)) = chars.peek().copied() {
                if peek_idx < next_idx {
                    chars.next();
                } else {
                    break;
                }
            }
            continue;
        }

        if is_word_char(ch) {
            current.push(ch);
            continue;
        }

        flush_word(language, &mut current, spans);
        let style = if is_operator_char(ch) {
            operator_style()
        } else {
            default_style()
        };
        spans.push(Span::styled(ch.to_string(), style));
    }
    flush_word(language, &mut current, spans);
}

fn flush_word(language: CodeLanguage, current: &mut String, spans: &mut Vec<Span<'static>>) {
    if current.is_empty() {
        return;
    }
    let word = std::mem::take(current);
    let style = token_style(language, &word);
    spans.push(Span::styled(word, style));
}

fn token_style(language: CodeLanguage, token: &str) -> Style {
    if is_number(token) {
        return number_style();
    }
    if is_keyword(language, token) {
        return keyword_style();
    }
    if is_type_keyword(language, token) || looks_like_type(token) {
        return type_style();
    }
    if is_builtin(language, token) {
        return function_style();
    }
    if matches!(
        language,
        CodeLanguage::Json | CodeLanguage::Yaml | CodeLanguage::Toml
    ) {
        return property_style();
    }
    default_style()
}

fn highlight_html_line(line: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut remaining = line;
    while let Some(start) = remaining.find('<') {
        if start > 0 {
            push_code_segment(CodeLanguage::Html, &remaining[..start], &mut spans);
        }
        if let Some(end) = remaining[start..].find('>') {
            let tag_text = &remaining[start..start + end + 1];
            spans.push(Span::styled(tag_text.to_string(), tag_style()));
            remaining = &remaining[start + end + 1..];
        } else {
            spans.push(Span::styled(remaining[start..].to_string(), tag_style()));
            remaining = "";
            break;
        }
    }
    if !remaining.is_empty() {
        push_code_segment(CodeLanguage::Html, remaining, &mut spans);
    }
    Line::from(spans)
}
