pub(crate) fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

pub(crate) fn char_display_width(c: char) -> usize {
    if c == '\t' {
        return 4;
    }
    if c.is_control() {
        return 0;
    }

    let code = c as u32;
    if matches!(
        code,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x3FFFD
    ) {
        2
    } else {
        1
    }
}

pub(crate) fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let ellipsis = "…";
    let ellipsis_width = display_width(ellipsis);
    let target_width = max_width.saturating_sub(ellipsis_width);
    let mut result = String::new();
    let mut used = 0usize;

    for ch in text.chars() {
        let width = char_display_width(ch);
        if used + width > target_width {
            break;
        }
        result.push(ch);
        used += width;
    }

    result.push_str(ellipsis);
    result
}
