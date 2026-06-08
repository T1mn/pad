mod range;
mod rows;

use crate::app::App;
use range::{normalized_selection_points, slice_text_by_width};
use ratatui::layout::Rect;

pub(super) use rows::preview_visible_plain_text_rows;

pub fn extract_preview_selection_text(
    app: &mut App,
    area: Rect,
    anchor: (u16, u16),
    current: (u16, u16),
) -> Option<String> {
    if area.width == 0 || area.height == 0 {
        return None;
    }

    let rows = preview_visible_plain_text_rows(app, area);
    if rows.is_empty() {
        return None;
    }

    let (start, end) = normalized_selection_points(area, anchor, current);
    if start == end {
        return None;
    }

    let start_row = start.1 as usize;
    let end_row = end.1 as usize;
    let mut parts = Vec::new();

    for row_idx in start_row..=end_row.min(rows.len().saturating_sub(1)) {
        let text = rows.get(row_idx).map(String::as_str).unwrap_or("");
        let piece = if start_row == end_row {
            slice_text_by_width(text, start.0 as usize, end.0 as usize + 1)
        } else if row_idx == start_row {
            slice_text_by_width(text, start.0 as usize, usize::MAX)
        } else if row_idx == end_row {
            slice_text_by_width(text, 0, end.0 as usize + 1)
        } else {
            text.to_string()
        };
        parts.push(piece);
    }

    let joined = parts.join("\n");
    if joined.trim().is_empty() {
        None
    } else {
        Some(joined)
    }
}
