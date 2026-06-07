use crate::app::App;
use ratatui::layout::Rect;

pub(super) fn update_hovered_folder(app: &mut App, terminal_area: Rect, column: u16, row: u16) {
    if app.should_defer_ui_updates() || app.frame_budget_exceeded {
        return;
    }

    let regions = super::normal_mouse_regions(app, terminal_area);
    let hovered_folder_key = if super::hit_test::rect_contains(regions.panel_inner, column, row) {
        let table_offset = app.table_state.offset();
        let items = app.visible_sidebar_items_ref();
        super::hit_test::panel_index_at_position(regions.panel_inner, row, table_offset, items)
            .and_then(|index| items.get(index).cloned())
            .and_then(|item| item.as_folder().map(|folder| folder.key.clone()))
    } else {
        None
    };

    if hovered_folder_key != app.sidebar.hovered_folder_key {
        app.sidebar.hovered_folder_key = hovered_folder_key;
        app.dirty = true;
    }
}
