use super::language::{comment_prefixes, CodeLanguage};

pub(super) fn comment_start(language: CodeLanguage, line: &str) -> Option<usize> {
    let mut in_string = None;
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if let Some(quote) = in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            continue;
        }
        if is_quote(language, ch) {
            in_string = Some(ch);
            continue;
        }
        for prefix in comment_prefixes(language) {
            if line[idx..].starts_with(prefix) {
                return Some(idx);
            }
        }
    }
    None
}

pub(super) fn consume_string(segment: &str, start: usize, quote: char) -> (&str, usize) {
    let mut escaped = false;
    for (idx, ch) in segment[start + quote.len_utf8()..].char_indices() {
        let absolute = start + quote.len_utf8() + idx;
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            let end = absolute + ch.len_utf8();
            return (&segment[start..end], end);
        }
    }
    (&segment[start..], segment.len())
}

pub(super) fn is_quote(language: CodeLanguage, ch: char) -> bool {
    match language {
        CodeLanguage::Shell => matches!(ch, '\'' | '"' | '`'),
        CodeLanguage::Json => ch == '"',
        _ => matches!(ch, '\'' | '"' | '`'),
    }
}

pub(super) fn is_word_char(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphanumeric()
}

pub(super) fn is_operator_char(ch: char) -> bool {
    matches!(
        ch,
        '+' | '-'
            | '*'
            | '/'
            | '%'
            | '='
            | '!'
            | '<'
            | '>'
            | '&'
            | '|'
            | '^'
            | '~'
            | ':'
            | '?'
            | '.'
    )
}

pub(super) fn is_number(token: &str) -> bool {
    token.chars().any(|ch| ch.is_ascii_digit())
        && token
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() || matches!(ch, '_' | '.' | 'x' | 'X' | 'b' | 'B'))
}

pub(super) fn looks_like_type(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
        && token.chars().any(|ch| ch.is_ascii_lowercase())
}
