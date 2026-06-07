use crate::app::App;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

mod click;
mod hit_test;
mod hover;
mod regions;
mod scroll;
mod selection;

pub(in crate::event) use regions::normal_mouse_regions;
#[cfg(test)]
pub(in crate::event) use scroll::MOUSE_PREVIEW_SCROLL_DELTA;
pub(in crate::event) use scroll::{
    coalesce_scroll_burst, drain_pending_scroll_events, handle_normal_scroll, mouse_scroll_delta,
};

pub(super) fn handle_normal_mouse(app: &mut App, terminal_area: Rect, mouse: MouseEvent) {
    if app.sidebar.show_tree {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.clear_panel_tab();
            let _ = app.clear_preview_mouse_selection();
            click::handle_normal_left_click(app, terminal_area, mouse.column, mouse.row);
        }
        MouseEventKind::Drag(MouseButton::Left) if app.preview.mouse_selection().is_some() => {
            selection::update_preview_mouse_selection(app, terminal_area, mouse.column, mouse.row);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            selection::finish_preview_mouse_selection(app, terminal_area, mouse.column, mouse.row);
        }
        MouseEventKind::Moved => {
            if app.preview.mouse_selection().is_some() {
                selection::update_preview_mouse_selection(
                    app,
                    terminal_area,
                    mouse.column,
                    mouse.row,
                );
                return;
            }
            hover::update_hovered_folder(app, terminal_area, mouse.column, mouse.row);
        }
        MouseEventKind::ScrollUp => {
            app.clear_panel_tab();
            let _ = app.clear_preview_mouse_selection();
            scroll::handle_normal_scroll(app, terminal_area, mouse.column, mouse.row, -1);
        }
        MouseEventKind::ScrollDown => {
            app.clear_panel_tab();
            let _ = app.clear_preview_mouse_selection();
            scroll::handle_normal_scroll(app, terminal_area, mouse.column, mouse.row, 1);
        }
        _ => {}
    }
}
