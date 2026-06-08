use ratatui::layout::{Constraint, Direction, Layout, Rect};

const MIN_LEFT_WIDTH: u16 = 34;
const MAX_LEFT_WIDTH: u16 = 46;
const MIN_PREVIEW_WIDTH: u16 = 30;
const MIN_TINY_LEFT_WIDTH: u16 = 24;

pub fn split_columns(area: Rect) -> (Rect, Rect) {
    let left_width = left_column_width(area.width);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(left_width), Constraint::Min(0)])
        .split(area);
    (columns[0], columns[1])
}

pub fn left_column_width(total_width: u16) -> u16 {
    let max_left_for_preview = total_width.saturating_sub(MIN_PREVIEW_WIDTH);
    if max_left_for_preview < MIN_LEFT_WIDTH {
        return max_left_for_preview
            .max(MIN_TINY_LEFT_WIDTH)
            .min(total_width);
    }

    let preferred = ((total_width as u32 * 32) / 100) as u16;
    preferred
        .clamp(MIN_LEFT_WIDTH, MAX_LEFT_WIDTH)
        .min(max_left_for_preview)
}

#[cfg(test)]
#[path = "split_tests.rs"]
mod tests;
