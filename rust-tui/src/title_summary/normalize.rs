use crate::text_normalize::collapse_whitespace;

const MAX_TITLE_CHARS: usize = 60;

pub fn normalize_generated_title(raw: &str) -> Option<String> {
    let single_line = raw.trim().lines().next()?.trim();
    if single_line.is_empty() {
        return None;
    }

    let mut normalized = collapse_whitespace(single_line);
    normalized = strip_known_prefix(&normalized).to_string();

    while let Some(stripped) = strip_matching_wrappers(&normalized) {
        normalized = stripped.to_string();
    }

    if normalized.is_empty() {
        return None;
    }

    let mut clipped = String::new();
    for (idx, ch) in normalized.chars().enumerate() {
        if idx >= MAX_TITLE_CHARS {
            break;
        }
        clipped.push(ch);
    }

    let clipped = clipped.trim();
    if clipped.is_empty() {
        None
    } else {
        Some(clipped.to_string())
    }
}

fn strip_known_prefix(value: &str) -> &str {
    let trimmed = value.trim();
    for prefix in ["title:", "Title:", "标题:", "題名:", "标题：", "題名："] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let rest = rest.trim();
            if !rest.is_empty() {
                return rest;
            }
        }
    }
    trimmed
}

fn strip_matching_wrappers(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    let pairs = [
        ('"', '"'),
        ('\'', '\''),
        ('`', '`'),
        ('“', '”'),
        ('‘', '’'),
        ('「', '」'),
        ('『', '』'),
        ('《', '》'),
        ('〈', '〉'),
    ];

    for (left, right) in pairs {
        if trimmed.starts_with(left) && trimmed.ends_with(right) && trimmed.len() > 1 {
            let start = left.len_utf8();
            let end = trimmed.len().saturating_sub(right.len_utf8());
            if start < end {
                return Some(trimmed[start..end].trim());
            }
        }
    }
    None
}
