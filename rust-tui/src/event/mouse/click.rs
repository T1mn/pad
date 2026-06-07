use crate::app::App;
use crate::{model::PreviewView, ui};
use ratatui::layout::Rect;

pub(super) fn handle_normal_left_click(app: &mut App, terminal_area: Rect, column: u16, row: u16) {
    let regions = super::normal_mouse_regions(app, terminal_area);

    if super::hit_test::rect_contains(regions.panel_area, column, row) {
        click_panel(app, regions.panel_inner, column, row);
        app.focus_panel();
        return;
    }

    if !super::hit_test::rect_contains(regions.preview_area, column, row) || !app.focus_preview() {
        return;
    }

    if click_preview_info(app, regions.preview_info_area, column, row) {
        return;
    }

    if click_preview_turn(app, regions.preview_content_area, column, row) {
        return;
    }

    if super::hit_test::rect_contains(regions.preview_content_area, column, row)
        && preview_mouse_copy_enabled(app)
    {
        app.begin_preview_mouse_selection(column, row);
    }
}

fn click_panel(app: &mut App, panel_inner: Rect, column: u16, row: u16) {
    if !super::hit_test::rect_contains(panel_inner, column, row) {
        return;
    }

    let table_offset = app.table_state.offset();
    let items = app.visible_sidebar_items_ref();
    let Some(index) =
        super::hit_test::panel_index_at_position(panel_inner, row, table_offset, items)
    else {
        return;
    };

    let is_folder = items
        .get(index)
        .is_some_and(|item| item.as_folder().is_some());
    if is_folder {
        let _ = app.select_sidebar_index(index, false);
        let _ = app.toggle_selected_folder();
    } else {
        let _ = app.jump_to_sidebar_index(index);
    }
}

fn click_preview_info(app: &mut App, info_area: Option<Rect>, column: u16, row: u16) -> bool {
    let Some(info_area) = info_area else {
        return false;
    };
    if !super::hit_test::rect_contains(info_area, column, row) {
        return false;
    }

    if let Some(session_id) = ui::preview::preview_sid_text_at(app, info_area, column, row) {
        let _ = app.copy_text_with_toast("SID", &session_id);
    } else if let Some(share_url) =
        ui::preview::preview_share_url_text_at(app, info_area, column, row)
    {
        let _ = app.copy_text_with_toast("SHARE", &share_url);
    }
    true
}

fn click_preview_turn(app: &mut App, preview_content_area: Rect, column: u16, row: u16) -> bool {
    if !app.has_session_preview_turns()
        || app.preview.view != PreviewView::SessionList
        || !super::hit_test::rect_contains(preview_content_area, column, row)
    {
        return false;
    }

    if let Some(index) = super::hit_test::session_turn_index_at_position(
        preview_content_area,
        row,
        app.preview.list_scroll,
        app.preview.turns.len(),
    ) {
        if app.preview.selected_turn == Some(index) {
            let _ = app.toggle_preview_turn_expanded();
        } else {
            let _ = app.select_preview_turn(index);
        }
    }
    true
}

fn preview_mouse_copy_enabled(app: &App) -> bool {
    !(app.has_session_preview_turns() && app.preview.view == PreviewView::SessionList)
}
