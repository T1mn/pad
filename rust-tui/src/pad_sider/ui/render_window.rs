use ratatui::text::Line;

pub(super) fn visible_line_window(
    lines: &[Line<'_>],
    viewport_width: usize,
    scroll: u16,
    viewport_height: usize,
) -> (std::ops::Range<usize>, u16) {
    if lines.is_empty() || viewport_width == 0 || viewport_height == 0 {
        return (0..0, 0);
    }

    let scroll = scroll as usize;
    let mut row_start = 0usize;
    let mut start_idx = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        let rows = line_wrapped_rows(line, viewport_width);
        if row_start + rows > scroll {
            start_idx = idx;
            break;
        }
        row_start += rows;
        start_idx = (idx + 1).min(lines.len());
    }

    if start_idx >= lines.len() {
        return (lines.len()..lines.len(), 0);
    }

    let local_scroll = scroll.saturating_sub(row_start).min(u16::MAX as usize) as u16;
    let target_rows = viewport_height.saturating_add(local_scroll as usize);
    let mut covered_rows = 0usize;
    let mut end_idx = start_idx;
    for line in &lines[start_idx..] {
        covered_rows += line_wrapped_rows(line, viewport_width);
        end_idx += 1;
        if covered_rows >= target_rows {
            break;
        }
    }

    (start_idx..end_idx, local_scroll)
}

fn line_wrapped_rows(line: &Line<'_>, viewport_width: usize) -> usize {
    let width = line
        .spans
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum::<usize>();
    if width == 0 {
        1
    } else {
        width.div_ceil(viewport_width).max(1)
    }
}

fn display_width(text: &str) -> usize {
    if text.bytes().all(|byte| (b' '..=b'~').contains(&byte)) {
        return text.len();
    }
    text.chars().map(char_display_width).sum()
}

fn char_display_width(ch: char) -> usize {
    if ch == '\t' {
        return 4;
    }
    if ch.is_control() {
        return 0;
    }

    let code = ch as u32;
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

#[cfg(test)]
#[path = "render_window_tests.rs"]
mod tests;
