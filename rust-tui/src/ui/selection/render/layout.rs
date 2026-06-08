use ratatui::layout::{Margin, Rect};

const SELECTION_HORIZONTAL_PADDING: u16 = 2;
const SELECTION_VERTICAL_PADDING: u16 = 1;

pub const fn selection_surface_padding_height() -> u16 {
    SELECTION_VERTICAL_PADDING * 2
}

pub fn recommended_list_modal_height(
    item_count: u16,
    row_height: u16,
    header_lines: u16,
    footer_lines: u16,
) -> u16 {
    item_count
        .max(1)
        .saturating_mul(row_height)
        .saturating_add(header_lines)
        .saturating_add(footer_lines)
        .saturating_add(selection_surface_padding_height())
}

pub(super) fn padded_inner(area: Rect) -> Rect {
    let horizontal = SELECTION_HORIZONTAL_PADDING.min(area.width.saturating_sub(1) / 2);
    let vertical = SELECTION_VERTICAL_PADDING.min(area.height.saturating_sub(1) / 2);
    area.inner(Margin {
        horizontal,
        vertical,
    })
}
