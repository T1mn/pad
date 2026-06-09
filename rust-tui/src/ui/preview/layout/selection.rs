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
    let mut selected = String::new();
    let mut wrote_piece = false;

    for row_idx in start_row..=end_row.min(rows.len().saturating_sub(1)) {
        let text = rows.get(row_idx).map(String::as_str).unwrap_or("");
        if wrote_piece {
            selected.push('\n');
        } else if row_idx == start_row {
            wrote_piece = true;
        }

        if start_row == end_row {
            selected.push_str(&slice_text_by_width(
                text,
                start.0 as usize,
                end.0 as usize + 1,
            ));
        } else if row_idx == start_row {
            selected.push_str(&slice_text_by_width(text, start.0 as usize, usize::MAX));
        } else if row_idx == end_row {
            selected.push_str(&slice_text_by_width(text, 0, end.0 as usize + 1));
        } else {
            selected.push_str(text);
        }
    }

    if selected.trim().is_empty() {
        None
    } else {
        Some(selected)
    }
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
