use super::super::app::{App, Focus};
use super::super::search::SearchAction;
use super::super::sizing;
use super::split;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

const MOUSE_SCROLL_LINES: u16 = 3;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if app.show_help {
        match key.code {
            KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc => app.close_help(),
            _ => {}
        }
        return;
    }

    if app.preview.is_some() {
        handle_preview_key(app, key);
        return;
    }

    if app.search.is_some() {
        handle_search_key(app, key);
        return;
    }

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Char('j') | KeyCode::Down => app.next(),
        KeyCode::Char('k') | KeyCode::Up => app.previous(),
        KeyCode::Char('r') => app.refresh(),
        KeyCode::Char('n') => app.toggle_line_numbers(),
        KeyCode::Char('=') | KeyCode::Char('+') => app.zoom_text_in(),
        KeyCode::Char('-') => app.zoom_text_out(),
        KeyCode::Char('{') => app.shrink_focused_section(),
        KeyCode::Char('}') => app.grow_focused_section(),
        KeyCode::Char('[') => resize_sider(app, false),
        KeyCode::Char(']') => resize_sider(app, true),
        KeyCode::Char('0') => app.reset_layout(),
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::Char('t') => app.focus_tree(),
        KeyCode::Char('p') => app.focus_preview(),
        KeyCode::Char('I') => app.press_index_toggle_key(),
        KeyCode::Char('d') => app.focus_changes(),
        KeyCode::PageDown => app.file_preview_down(),
        KeyCode::PageUp => app.file_preview_up(),
        KeyCode::Enter if app.focus == Focus::Tree => app.toggle_selected(),
        KeyCode::Enter | KeyCode::Char(' ') if app.focus == Focus::IndexMap => {
            app.open_selected_index_preview()
        }
        KeyCode::Char(' ') if app.focus == Focus::Tree => handle_tree_space(app),
        KeyCode::Char('/') if app.focus == Focus::Tree => app.open_search(),
        KeyCode::Char('i') if app.focus == Focus::Tree => app.open_nearest_index_preview(),
        KeyCode::Char('o') if app.focus == Focus::IndexMap => app.reveal_selected_index_in_tree(),
        KeyCode::Char('g') => app.reset_position(),
        KeyCode::Char('G') => app.jump_bottom(),
        _ => {}
    }
}

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

    let left_height = area.height.saturating_sub(6);
    let nav_weight = app.active_nav_weight();
    let changes_weight = app.layout_weights.changes;
    let total = nav_weight + changes_weight;
    let nav_height = if total == 0 {
        left_height
    } else {
        (left_height as u32 * nav_weight as u32 / total as u32) as u16
    };

    if mouse.row.saturating_sub(6) < nav_height {
        app.focus_active_nav();
        scroll_nav(app, down);
    } else {
        app.focus_changes();
        scroll_changes(app, down);
    }
}

fn resize_sider(app: &App, wider: bool) {
    let _ = sizing::resize_from_helper(app.target_pane.as_deref(), wider);
}

fn handle_tree_space(app: &mut App) {
    if app.selected_is_dir() {
        app.toggle_selected();
    } else if app.selected_is_markdown() {
        app.open_preview();
    }
}

fn handle_preview_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.close_preview(),
        KeyCode::Char('j') | KeyCode::Down => app.preview_down(),
        KeyCode::Char('k') | KeyCode::Up => app.preview_up(),
        KeyCode::Char('n') => app.toggle_line_numbers(),
        KeyCode::Char('=') | KeyCode::Char('+') => app.zoom_text_in(),
        KeyCode::Char('-') => app.zoom_text_out(),
        KeyCode::Char('g') => app.reset_preview(),
        KeyCode::Char('G') => app.preview_bottom(),
        KeyCode::Char('?') => app.toggle_help(),
        _ => {}
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) {
    let action = app
        .search
        .as_mut()
        .map(|search| search.handle_key(key))
        .unwrap_or(SearchAction::Cancel);

    match action {
        SearchAction::None => {}
        SearchAction::Cancel => app.close_search(),
        SearchAction::Submit(path) => {
            app.close_search();
            app.reveal_path(&path);
        }
    }
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

fn scroll_changes(app: &mut App, down: bool) {
    if down {
        app.changes_scroll = app.changes_scroll.saturating_add(MOUSE_SCROLL_LINES);
    } else {
        app.changes_scroll = app.changes_scroll.saturating_sub(MOUSE_SCROLL_LINES);
    }
}
