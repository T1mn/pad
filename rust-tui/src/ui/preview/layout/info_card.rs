mod hit;
mod render;
mod values;

use crate::app::App;
use crate::sidebar::SidebarThread;
use crate::theme::Theme;
use ratatui::{layout::Rect, Frame};

use render::PREVIEW_INFO_LABEL_WIDTH;

pub(crate) fn draw_preview_info_card(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    thread: &SidebarThread,
) {
    let value_width = area
        .width
        .saturating_sub(2)
        .saturating_sub((PREVIEW_INFO_LABEL_WIDTH + 1) as u16) as usize;
    let values = values::build_info_card_values(app, thread, value_width);
    render::render_info_card(f, area, theme, thread, &values);
}

pub fn preview_sid_text_at(app: &mut App, area: Rect, column: u16, row: u16) -> Option<String> {
    hit::preview_sid_text_at(app, area, column, row)
}

pub fn preview_share_url_text_at(
    app: &mut App,
    area: Rect,
    column: u16,
    row: u16,
) -> Option<String> {
    hit::preview_share_url_text_at(app, area, column, row)
}

#[cfg(test)]
pub(super) use hit::preview_info_value_text_at;
