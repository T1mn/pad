use ratatui::layout::Rect;

pub(super) fn normalized_selection_points(
    area: Rect,
    anchor: (u16, u16),
    current: (u16, u16),
) -> ((u16, u16), (u16, u16)) {
    let start = clamped_point_in_area(area, anchor);
    let end = clamped_point_in_area(area, current);
    if (start.1, start.0) <= (end.1, end.0) {
        (start, end)
    } else {
        (end, start)
    }
}

fn clamped_point_in_area(area: Rect, point: (u16, u16)) -> (u16, u16) {
    let max_x = area.width.saturating_sub(1);
    let max_y = area.height.saturating_sub(1);
    let x = point.0.saturating_sub(area.x).min(max_x);
    let y = point.1.saturating_sub(area.y).min(max_y);
    (x, y)
}

pub(super) fn slice_text_by_width(text: &str, start: usize, end: usize) -> String {
    if start >= end {
        return String::new();
    }

    let mut out = String::new();
    let mut offset = 0usize;
    for ch in text.chars() {
        let width = super::super::super::common::char_display_width(ch).max(1);
        let ch_start = offset;
        let ch_end = offset + width;
        if ch_end > start && ch_start < end {
            out.push(ch);
        }
        offset = ch_end;
        if offset >= end {
            break;
        }
    }
    out
}
