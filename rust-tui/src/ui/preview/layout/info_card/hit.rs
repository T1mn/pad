use super::super::super::common::{display_width, truncate_to_width};
use super::PREVIEW_INFO_LABEL_WIDTH;
use crate::app::App;
use ratatui::{
    layout::Rect,
    widgets::{Block, BorderType, Borders},
};

pub(super) fn preview_sid_text_at(
    app: &mut App,
    area: Rect,
    column: u16,
    row: u16,
) -> Option<String> {
    let thread = app.selected_preview_thread()?;
    let session_id = app
        .preview
        .session_id
        .as_deref()
        .or(thread.session_id.as_deref())?;
    preview_info_value_text_at(area, column, row, 4, session_id)
}

pub(super) fn preview_share_url_text_at(
    app: &mut App,
    area: Rect,
    column: u16,
    row: u16,
) -> Option<String> {
    let thread = app.selected_preview_thread()?;
    let share_url = thread.share_url.as_deref()?;
    preview_info_value_text_at(area, column, row, 7, share_url)
}

pub(in crate::ui::preview::layout) fn preview_info_value_text_at(
    area: Rect,
    column: u16,
    row: u16,
    line_offset: u16,
    value: &str,
) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value == "—" || area.width < 3 || area.height < 3 {
        return None;
    }

    let inner = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .inner(area);
    let target_row = inner.y.saturating_add(line_offset);
    if row != target_row {
        return None;
    }

    let label_width = PREVIEW_INFO_LABEL_WIDTH as u16;
    let value_x = inner.x.saturating_add(label_width + 1);
    let max_width = inner.width.saturating_sub(label_width + 1) as usize;
    let visible = truncate_to_width(value, max_width);
    let value_width = display_width(&visible) as u16;

    if column >= value_x && column < value_x.saturating_add(value_width.max(1)) {
        Some(value.to_string())
    } else {
        None
    }
}
