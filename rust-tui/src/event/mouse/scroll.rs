use crate::app::App;
use crossterm::event::{self, Event, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::io;
use std::time::Duration;

pub(in crate::event) const MOUSE_PREVIEW_SCROLL_DELTA: i32 = 3;

pub(in crate::event) fn handle_normal_scroll(
    app: &mut App,
    terminal_area: Rect,
    column: u16,
    row: u16,
    delta: i32,
) {
    let regions = super::normal_mouse_regions(app, terminal_area);

    if super::hit_test::rect_contains(regions.panel_area, column, row) {
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

    if super::hit_test::rect_contains(regions.preview_area, column, row) && app.focus_preview() {
        app.scroll_preview_by(delta * MOUSE_PREVIEW_SCROLL_DELTA);
    }
}

pub(in crate::event) fn mouse_scroll_delta(kind: &MouseEventKind) -> Option<i32> {
    match kind {
        MouseEventKind::ScrollUp => Some(-1),
        MouseEventKind::ScrollDown => Some(1),
        _ => None,
    }
}

pub(in crate::event) fn coalesce_scroll_burst(
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

pub(in crate::event) fn drain_pending_scroll_events(
    carried_event: &mut Option<Event>,
) -> io::Result<usize> {
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
