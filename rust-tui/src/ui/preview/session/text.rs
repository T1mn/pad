use super::super::common::{char_display_width, truncate_to_width};

pub(super) fn split_preview_card_lines(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut remaining = text.trim();
    let mut lines = Vec::with_capacity(max_lines);

    for idx in 0..max_lines {
        if remaining.is_empty() {
            lines.push(String::new());
            continue;
        }

        if idx + 1 == max_lines {
            lines.push(truncate_to_width(remaining, width));
            remaining = "";
            continue;
        }

        let (prefix, rest) = take_prefix_by_width(remaining, width);
        lines.push(prefix.to_string());
        remaining = rest;
    }

    lines
}

fn take_prefix_by_width(text: &str, max_width: usize) -> (&str, &str) {
    if max_width == 0 || text.is_empty() {
        return ("", text);
    }

    let mut used = 0usize;
    let mut split_at = text.len();
    for (idx, ch) in text.char_indices() {
        let ch_width = char_display_width(ch);
        if used + ch_width > max_width {
            split_at = idx;
            break;
        }
        used += ch_width;
    }

    if split_at == text.len() {
        return (text, "");
    }

    let prefix = text[..split_at].trim_end();
    let rest = text[split_at..].trim_start();
    (prefix, rest)
}

pub(super) fn question_text_for_display(text: &str) -> String {
    strip_turn_prefix(text, &["Q:", "Q：", "Question:", "question:"]).to_string()
}

pub(super) fn answer_text_for_display(text: &str) -> String {
    strip_turn_prefix(text, &["A:", "A：", "Answer:", "answer:"]).to_string()
}

fn strip_turn_prefix<'a>(text: &'a str, prefixes: &[&str]) -> &'a str {
    let trimmed = text.trim();
    for prefix in prefixes {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.trim_start();
        }
    }
    trimmed
}
