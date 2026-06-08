use super::super::super::app::App;
use super::super::split;
use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

const MOUSE_SCROLL_LINES: u16 = 3;

pub fn handle_mouse(app: &mut App, area: Rect, mouse: MouseEvent) {
    if app.show_help || app.search.is_some() {
        return;
    }

    let Some(down) = scroll_down(&mouse.kind) else {
        return;
    };

    if app.preview.is_some() {
        scroll_full_preview(app, down);
        return;
    }

    let right_start = split::left_column_width(area.width);
    if mouse.column >= right_start {
        scroll_file_preview(app, down);
        app.focus_preview();
        return;
    }

    if mouse.row < 6 {
        return;
    }

    app.focus_active_nav();
    scroll_nav(app, down);
}

fn scroll_down(kind: &MouseEventKind) -> Option<bool> {
    match kind {
        MouseEventKind::ScrollDown => Some(true),
        MouseEventKind::ScrollUp => Some(false),
        _ => None,
    }
}

fn scroll_full_preview(app: &mut App, down: bool) {
    for _ in 0..MOUSE_SCROLL_LINES {
        if down {
            app.preview_down();
        } else {
            app.preview_up();
        }
    }
}

fn scroll_file_preview(app: &mut App, down: bool) {
    if down {
        app.file_preview_scroll_down(MOUSE_SCROLL_LINES);
    } else {
        app.file_preview_scroll_up(MOUSE_SCROLL_LINES);
    }
}

fn scroll_nav(app: &mut App, down: bool) {
    for _ in 0..MOUSE_SCROLL_LINES {
        if down {
            app.next();
        } else {
            app.previous();
        }
    }
}
