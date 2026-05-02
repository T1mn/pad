mod markdown;
mod overlay;
mod render;

use super::app::{App, Focus};
use super::search::SearchAction;
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
        KeyCode::Char('j') | KeyCode::Down => app.next(),
        KeyCode::Char('k') | KeyCode::Up => app.previous(),
        KeyCode::Char('r') => app.refresh(),
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::Enter if app.focus == Focus::Tree => app.toggle_selected(),
        KeyCode::Char(' ') if app.focus == Focus::Tree => handle_tree_space(app),
        KeyCode::Char('/') if app.focus == Focus::Tree => app.open_search(),
        KeyCode::Char('g') => app.reset_position(),
        _ => {}
    }
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
