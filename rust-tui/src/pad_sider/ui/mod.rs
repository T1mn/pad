mod line_numbers;
mod markdown;
mod overlay;
mod render;

use super::app::{App, Focus};
use super::search::SearchAction;
use super::sizing;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

pub fn run(cwd: PathBuf, target_pane: Option<String>) -> Result<(), String> {
    enable_raw_mode().map_err(|err| err.to_string())?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|err| err.to_string())?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|err| err.to_string())?;
    let mut app = App::new(cwd, target_pane);

    let result = loop {
        app.tick();
        terminal
            .draw(|frame| render::draw(frame, &app))
            .map_err(|err| err.to_string())?;
        if app.should_quit {
            break Ok(());
        }
        if event::poll(Duration::from_millis(200)).map_err(|err| err.to_string())? {
            if let Event::Key(key) = event::read().map_err(|err| err.to_string())? {
                handle_key(&mut app, key);
            }
        }
    };

    disable_raw_mode().map_err(|err| err.to_string())?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|err| err.to_string())?;
    terminal.show_cursor().map_err(|err| err.to_string())?;
    result
}

fn handle_key(app: &mut App, key: KeyEvent) {
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
        KeyCode::Char('[') => resize_sider(app, false),
        KeyCode::Char(']') => resize_sider(app, true),
        KeyCode::Char('+') | KeyCode::Char('=') => app.grow_focused_section(),
        KeyCode::Char('-') => app.shrink_focused_section(),
        KeyCode::Char('0') => app.reset_layout(),
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::Char('t') => app.focus_tree(),
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
