use crate::app::App;
use crate::ui;
use ratatui::layout::Rect;

pub(super) fn update_preview_mouse_selection(
    app: &mut App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) {
    let regions = super::normal_mouse_regions(app, terminal_area);
    let (column, row) = clamp_to_preview_content(regions.preview_content_area, column, row);
    let _ = app.update_preview_mouse_selection(column, row);
}

pub(super) fn finish_preview_mouse_selection(
    app: &mut App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) {
    let Some(selection) = app.finish_preview_mouse_selection() else {
        return;
    };

    let regions = super::normal_mouse_regions(app, terminal_area);
    if let Some(text) = ui::preview::extract_preview_selection_text(
        app,
        regions.preview_content_area,
        (selection.anchor_column, selection.anchor_row),
        (column, row),
    ) {
        let _ = app.copy_text_with_toast("内容", &text);
    }
}

fn clamp_to_preview_content(area: Rect, column: u16, row: u16) -> (u16, u16) {
    (
        column.clamp(area.x, area.right().saturating_sub(1)),
        row.clamp(area.y, area.bottom().saturating_sub(1)),
    )
}
