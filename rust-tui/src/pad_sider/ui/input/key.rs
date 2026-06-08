use super::super::super::app::{App, Focus};
use super::super::super::search::SearchAction;
use super::super::super::sizing;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if app.show_help {
        handle_help_key(app, key);
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

    handle_main_key(app, key);
}

fn handle_help_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc => app.close_help(),
        _ => {}
    }
}

fn handle_main_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Char('j') | KeyCode::Down => app.next(),
        KeyCode::Char('k') | KeyCode::Up => app.previous(),
        KeyCode::Char('r') => app.refresh(),
        KeyCode::Char('n') => app.toggle_line_numbers(),
        KeyCode::Char('=') | KeyCode::Char('+') => app.zoom_text_in(),
        KeyCode::Char('-') => app.zoom_text_out(),
        KeyCode::Char('[') => resize_sider(app, false),
        KeyCode::Char(']') => resize_sider(app, true),
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::Char('t') => app.focus_tree(),
        KeyCode::Char('p') => app.focus_preview(),
        KeyCode::Char('c') => app.focus_codex_runs(),
        KeyCode::Char('I') => app.press_index_toggle_key(),
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

fn resize_sider(app: &App, wider: bool) {
    let _ = sizing::resize_from_helper(app.target_pane.as_deref(), wider);
}

fn handle_tree_space(app: &mut App) {
    if app.selected_is_dir() {
        app.toggle_selected();
    } else {
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
