use ratatui::{
    style::{Color, Style},
    text::Span,
};

pub(super) fn format_status_remainder(text: &str, width: u16, occupied: usize) -> String {
    let available = width as usize;
    let remaining = available.saturating_sub(occupied);
    if remaining <= 1 {
        return String::new();
    }
    let content_target = remaining.saturating_sub(1);
    let content = if display_width(text) <= content_target {
        text.to_string()
    } else {
        truncate_from_left_to_width(text, content_target)
    };
    format!(" {}", content)
}

pub(super) fn format_two_sided(left: &str, right: &str, width: usize) -> String {
    let right_width = display_width(right);
    if right_width + 2 >= width {
        return truncate_to_width(right, width);
    }
    let left_budget = width.saturating_sub(right_width + 3);
    let left_text = truncate_to_width(left, left_budget);
    format!("{}   {}", left_text, right)
}

pub(super) fn mode_badge(label: &str, bg: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default().fg(Color::Black).bg(bg),
    )
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
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

fn truncate_from_left_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let ellipsis = "…";
    let ellipsis_width = display_width(ellipsis);
    if max_width <= ellipsis_width {
        return ellipsis.to_string();
    }

    let keep_width = max_width.saturating_sub(ellipsis_width);
    let mut kept = Vec::new();
    let mut used = 0usize;

    for ch in text.chars().rev() {
        let width = char_display_width(ch);
        if used + width > keep_width {
            break;
        }
        kept.push(ch);
        used += width;
    }

    kept.reverse();
    let mut result = String::from(ellipsis);
    result.extend(kept);
    result
}

pub(super) fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

fn char_display_width(c: char) -> usize {
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
