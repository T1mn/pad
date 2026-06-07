use ratatui::layout::Rect;

pub(super) fn rect_contains(area: Rect, column: u16, row: u16) -> bool {
    area.width > 0
        && area.height > 0
        && column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

pub(super) fn panel_index_at_position(
    panel_inner: Rect,
    row: u16,
    table_offset: usize,
    items: &[crate::sidebar::SidebarItem],
) -> Option<usize> {
    if items.is_empty() || !rect_contains(panel_inner, panel_inner.x, row) {
        return None;
    }

    let mut remaining = row.saturating_sub(panel_inner.y) as usize;
    for (index, item) in items.iter().enumerate().skip(table_offset) {
        let height = sidebar_item_height(item) as usize;
        if remaining < height {
            return Some(index);
        }
        remaining = remaining.saturating_sub(height);
    }

    None
}

fn sidebar_item_height(item: &crate::sidebar::SidebarItem) -> u16 {
    if item.as_folder().is_some() {
        1
    } else {
        2
    }
}

pub(super) fn session_turn_index_at_position(
    preview_content_area: Rect,
    row: u16,
    scroll: u16,
    turn_count: usize,
) -> Option<usize> {
    if turn_count == 0 || !rect_contains(preview_content_area, preview_content_area.x, row) {
        return None;
    }

    let line = scroll as usize + row.saturating_sub(preview_content_area.y) as usize;
    crate::ui::preview::session_turn_index_at_line(line, turn_count)
}
