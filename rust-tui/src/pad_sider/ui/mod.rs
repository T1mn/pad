mod diff;
mod file_preview;
mod input;
mod line_numbers;
mod markdown;
mod nav_window;
mod overlay;
mod render;
mod render_window;
mod split;
mod text_zoom;

use super::app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

pub fn run(cwd: PathBuf, target_pane: Option<String>) -> Result<(), String> {
    enable_raw_mode().map_err(|err| err.to_string())?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|err| err.to_string())?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|err| err.to_string())?;
    let mut app = App::new(cwd, target_pane);

    let result = loop {
        app.tick();
        if app.should_quit {
            break Ok(());
        }
        if app.take_dirty() {
            let started_at = std::time::Instant::now();
            terminal
                .draw(|frame| render::draw(frame, &mut app))
                .map_err(|err| err.to_string())?;
            let elapsed = started_at.elapsed();
            if elapsed >= Duration::from_millis(8) {
                crate::log_debug!(
                    "pad_sider.frame: draw_slow elapsed_ms={} preview_bytes={} markdown={}",
                    elapsed.as_millis(),
                    app.file_preview.content.len(),
                    matches!(app.file_preview.kind, super::preview::PreviewKind::Markdown)
                );
            }
        }
        if event::poll(Duration::from_millis(200)).map_err(|err| err.to_string())? {
            match event::read().map_err(|err| err.to_string())? {
                Event::Key(key) => {
                    let should_redraw = key.kind == KeyEventKind::Press;
                    input::handle_key(&mut app, key);
                    if should_redraw {
                        app.mark_dirty();
                    }
                }
                Event::Mouse(mouse) => {
                    let should_redraw = matches!(
                        mouse.kind,
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                    );
                    let area = terminal_area(&mut terminal).map_err(|err| err.to_string())?;
                    input::handle_mouse(&mut app, area, mouse);
                    if should_redraw {
                        app.mark_dirty();
                    }
                }
                Event::Resize(_, _) => {
                    app.mark_dirty();
                }
                _ => {}
            }
        }
    };

    disable_raw_mode().map_err(|err| err.to_string())?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )
    .map_err(|err| err.to_string())?;
    terminal.show_cursor().map_err(|err| err.to_string())?;
    result
}

fn terminal_area(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<Rect> {
    let size = terminal.size()?;
    Ok(Rect::new(0, 0, size.width, size.height))
}
