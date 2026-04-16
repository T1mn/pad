use crate::app::App;
use crate::ui;
use crossterm::event::{self, Event, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::io;
use std::time::Duration;

pub(super) const MOUSE_PREVIEW_SCROLL_DELTA: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NormalMouseRegions {
    pub(super) panel_area: Rect,
    pub(super) panel_inner: Rect,
    pub(super) preview_area: Rect,
    pub(super) preview_inner: Rect,
    pub(super) preview_info_area: Option<Rect>,
    pub(super) preview_content_area: Rect,
}

fn inner_rect(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn rect_contains(area: Rect, column: u16, row: u16) -> bool {
    area.width > 0
        && area.height > 0
        && column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

pub(super) fn normal_mouse_regions(app: &mut App, terminal_area: Rect) -> NormalMouseRegions {
    let preferred_left_width = Some(ui::panel_list::preferred_panel_width(app));
    let (_, body_layout) = ui::layout::compute_layout(terminal_area, false, preferred_left_width);
    let panel_area = body_layout[0];
    let preview_area = body_layout[1];
    let panel_inner = inner_rect(panel_area);
    let preview_inner = inner_rect(preview_area);

    let (preview_info_area, preview_content_area) = if app.selected_preview_thread().is_some() {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(ui::preview::PREVIEW_INFO_CARD_HEIGHT),
                Constraint::Min(0),
            ])
            .split(preview_inner);
        (Some(split[0]), split[1])
    } else {
        (None, preview_inner)
    };

    NormalMouseRegions {
        panel_area,
        panel_inner,
        preview_area,
        preview_inner,
        preview_info_area,
        preview_content_area,
    }
}

fn panel_index_at_position(
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

fn session_turn_index_at_position(
    preview_content_area: Rect,
    row: u16,
    scroll: u16,
    turn_count: usize,
) -> Option<usize> {
    if turn_count == 0 || !rect_contains(preview_content_area, preview_content_area.x, row) {
        return None;
    }

    let line = scroll as usize + row.saturating_sub(preview_content_area.y) as usize;
    ui::preview::session_turn_index_at_line(line, turn_count)
}

fn handle_normal_left_click(app: &mut App, terminal_area: Rect, column: u16, row: u16) {
    let regions = normal_mouse_regions(app, terminal_area);

    if rect_contains(regions.panel_area, column, row) {
        if rect_contains(regions.panel_inner, column, row) {
            let table_offset = app.table_state.offset();
            let items = app.visible_sidebar_items_ref();
            if let Some(index) =
                panel_index_at_position(regions.panel_inner, row, table_offset, items)
            {
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
        }
        app.focus_panel();
        return;
    }

    if !rect_contains(regions.preview_area, column, row) {
        return;
    }

    if !app.focus_preview() {
        return;
    }

    if regions
        .preview_info_area
        .is_some_and(|info| rect_contains(info, column, row))
    {
        if let Some(info_area) = regions.preview_info_area {
            if let Some(session_id) = ui::preview::preview_sid_text_at(app, info_area, column, row)
            {
                let _ = app.copy_text_with_toast("SID", &session_id);
            }
        }
        return;
    }

    if app.has_session_preview_turns()
        && app.preview.view == crate::model::PreviewView::SessionList
        && rect_contains(regions.preview_content_area, column, row)
    {
        if let Some(index) = session_turn_index_at_position(
            regions.preview_content_area,
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
        return;
    }

    if rect_contains(regions.preview_content_area, column, row) && preview_mouse_copy_enabled(app) {
        app.begin_preview_mouse_selection(column, row);
    }
}

fn preview_mouse_copy_enabled(app: &App) -> bool {
    !(app.has_session_preview_turns() && app.preview.view == crate::model::PreviewView::SessionList)
}

pub(super) fn handle_normal_scroll(
    app: &mut App,
    terminal_area: Rect,
    column: u16,
    row: u16,
    delta: i32,
) {
    let regions = normal_mouse_regions(app, terminal_area);

    if rect_contains(regions.panel_area, column, row) {
        app.focus_panel();
        if !app.visible_sidebar_items_ref().is_empty() {
            if delta < 0 {
                app.previous();
            } else {
                app.next();
            }
        }
        return;
    }

    if rect_contains(regions.preview_area, column, row) && app.focus_preview() {
        app.scroll_preview_by(delta * MOUSE_PREVIEW_SCROLL_DELTA);
    }
}

pub(super) fn mouse_scroll_delta(kind: &MouseEventKind) -> Option<i32> {
    match kind {
        MouseEventKind::ScrollUp => Some(-1),
        MouseEventKind::ScrollDown => Some(1),
        _ => None,
    }
}

pub(super) fn coalesce_scroll_burst(
    first: MouseEvent,
    carried_event: &mut Option<Event>,
) -> io::Result<(u16, u16, i32)> {
    let mut column = first.column;
    let mut row = first.row;
    let mut delta = mouse_scroll_delta(&first.kind).unwrap_or_default();

    while event::poll(Duration::from_millis(0))? {
        let next = event::read()?;
        match next {
            Event::Mouse(mouse) if mouse_scroll_delta(&mouse.kind).is_some() => {
                column = mouse.column;
                row = mouse.row;
                delta += mouse_scroll_delta(&mouse.kind).unwrap_or_default();
            }
            other => {
                *carried_event = Some(other);
                break;
            }
        }
    }

    Ok((column, row, delta))
}

pub(super) fn drain_pending_scroll_events(carried_event: &mut Option<Event>) -> io::Result<usize> {
    let mut dropped = 0usize;
    while event::poll(Duration::from_millis(0))? {
        let next = event::read()?;
        match next {
            Event::Mouse(mouse) if mouse_scroll_delta(&mouse.kind).is_some() => {
                dropped += 1;
            }
            other => {
                *carried_event = Some(other);
                break;
            }
        }
    }
    Ok(dropped)
}

pub(super) fn handle_normal_mouse(app: &mut App, terminal_area: Rect, mouse: MouseEvent) {
    if app.sidebar.show_tree {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.clear_panel_tab();
            let _ = app.clear_preview_mouse_selection();
            handle_normal_left_click(app, terminal_area, mouse.column, mouse.row);
        }
        MouseEventKind::Drag(MouseButton::Left) if app.preview.mouse_selection().is_some() => {
            let regions = normal_mouse_regions(app, terminal_area);
            let column = mouse.column.clamp(
                regions.preview_content_area.x,
                regions.preview_content_area.right().saturating_sub(1),
            );
            let row = mouse.row.clamp(
                regions.preview_content_area.y,
                regions.preview_content_area.bottom().saturating_sub(1),
            );
            let _ = app.update_preview_mouse_selection(column, row);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if let Some(selection) = app.finish_preview_mouse_selection() {
                let regions = normal_mouse_regions(app, terminal_area);
                if let Some(text) = ui::preview::extract_preview_selection_text(
                    app,
                    regions.preview_content_area,
                    (selection.anchor_column, selection.anchor_row),
                    (mouse.column, mouse.row),
                ) {
                    let _ = app.copy_text_with_toast("内容", &text);
                }
            }
        }
        MouseEventKind::Moved => {
            if app.preview.mouse_selection().is_some() {
                let regions = normal_mouse_regions(app, terminal_area);
                let column = mouse.column.clamp(
                    regions.preview_content_area.x,
                    regions.preview_content_area.right().saturating_sub(1),
                );
                let row = mouse.row.clamp(
                    regions.preview_content_area.y,
                    regions.preview_content_area.bottom().saturating_sub(1),
                );
                let _ = app.update_preview_mouse_selection(column, row);
                return;
            }
            if app.should_defer_ui_updates() || app.frame_budget_exceeded {
                return;
            }
            let regions = normal_mouse_regions(app, terminal_area);
            let hovered_folder_key = if rect_contains(regions.panel_inner, mouse.column, mouse.row)
            {
                let table_offset = app.table_state.offset();
                let items = app.visible_sidebar_items_ref();
                panel_index_at_position(regions.panel_inner, mouse.row, table_offset, items)
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
        MouseEventKind::ScrollUp => {
            app.clear_panel_tab();
            let _ = app.clear_preview_mouse_selection();
            handle_normal_scroll(app, terminal_area, mouse.column, mouse.row, -1);
        }
        MouseEventKind::ScrollDown => {
            app.clear_panel_tab();
            let _ = app.clear_preview_mouse_selection();
            handle_normal_scroll(app, terminal_area, mouse.column, mouse.row, 1);
        }
        _ => {}
    }
}
