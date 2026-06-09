pub(crate) fn normalize_session_detail_markdown(text: &str) -> String {
    let mut lines = text.lines().peekable();
    let Some(first_line) = lines.next() else {
        return text.to_string();
    };
    if lines.peek().is_none() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut in_fenced_code = false;
    let mut line = first_line;
    let mut first = true;

    loop {
        if first {
            first = false;
        } else {
            out.push('\n');
        }
        out.push_str(line);

        let trimmed = line.trim();
        let can_insert_gap = if is_fence_marker(trimmed) {
            in_fenced_code = !in_fenced_code;
            false
        } else {
            !in_fenced_code
        };

        let Some(next) = lines.next() else {
            break;
        };
        if can_insert_gap && should_insert_session_paragraph_gap(line, next) {
            out.push('\n');
        }
        line = next;
    }

    out
}

fn should_insert_session_paragraph_gap(current: &str, next: &str) -> bool {
    let current = current.trim();
    let next = next.trim();
    if current.is_empty() || next.is_empty() {
        return false;
    }
    if is_fence_marker(current) || is_fence_marker(next) {
        return false;
    }
    if is_setext_underline(current) || is_setext_underline(next) {
        return false;
    }
    if is_markdown_structural_line(current) || is_markdown_structural_line(next) {
        return false;
    }
    true
}

fn is_fence_marker(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

fn is_setext_underline(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && trimmed.chars().all(|ch| matches!(ch, '-' | '='))
}

fn is_markdown_structural_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#')
        || trimmed.starts_with('>')
        || trimmed.starts_with('|')
        || trimmed.starts_with("    ")
        || trimmed.starts_with('\t')
        || trimmed.starts_with("- [")
        || trimmed.starts_with("* [")
        || trimmed.starts_with("+ [")
        || trimmed.starts_with("---")
        || trimmed.starts_with("***")
    {
        return true;
    }

    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return true;
    }

    let mut chars = trimmed.chars().peekable();
    let mut saw_digit = false;
    while let Some(ch) = chars.peek() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            chars.next();
        } else {
            break;
        }
    }
    if saw_digit && matches!(chars.next(), Some('.' | ')')) && matches!(chars.next(), Some(' ')) {
        return true;
    }

    false
}
