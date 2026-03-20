use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState, Wrap,
    },
    Frame, Terminal,
};
use std::{
    error::Error,
    io,
    process::Command,
    time::{Duration, Instant},
};
use tokio::time::interval;

mod model;
mod scanner;
mod ui;

use model::{CodePanel, CodeType};
use scanner::scan_panels;

/// Application state
struct App {
    /// List of panels
    panels: Vec<CodePanel>,
    /// Table state for selection
    table_state: TableState,
    /// Current mode
    mode: Mode,
    /// Last refresh time
    last_refresh: Instant,
    /// Auto refresh interval
    refresh_interval: Duration,
    /// Search query
    search_query: String,
    /// Is searching
    is_searching: bool,
    /// Preview content
    preview_content: String,
    /// Settings modal open
    settings_open: bool,
    /// Should quit
    should_quit: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Search,
    Settings,
}

impl App {
    fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        
        Self {
            panels: Vec::new(),
            table_state,
            mode: Mode::Normal,
            last_refresh: Instant::now(),
            refresh_interval: Duration::from_secs(2),
            search_query: String::new(),
            is_searching: false,
            preview_content: String::from("Select a panel to preview"),
            settings_open: false,
            should_quit: false,
        }
    }

    fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.filtered_panels().len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.update_preview();
    }

    fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_panels().len().saturating_sub(1)
                } else {
                    i.saturating_sub(1)
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.update_preview();
    }

    fn filtered_panels(&self) -> Vec<&CodePanel> {
        if self.search_query.is_empty() {
            self.panels.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.panels
                .iter()
                .filter(|p| {
                    p.session.to_lowercase().contains(&query)
                        || p.window.to_lowercase().contains(&query)
                        || p.working_dir.to_lowercase().contains(&query)
                })
                .collect()
        }
    }

    fn selected_panel(&self) -> Option<&CodePanel> {
        let filtered = self.filtered_panels();
        self.table_state
            .selected()
            .and_then(|i| filtered.get(i).copied())
    }

    fn update_preview(&mut self) {
        if let Some(panel) = self.selected_panel() {
            // Capture pane content
            match capture_pane(&panel.pane_id, 50) {
                Ok(content) => {
                    self.preview_content = content;
                }
                Err(_) => {
                    self.preview_content = String::from("Failed to capture pane");
                }
            }
        }
    }

    fn refresh_panels(&mut self) {
        match scan_panels() {
            Ok(panels) => {
                self.panels = panels;
                self.last_refresh = Instant::now();
                self.update_preview();
            }
            Err(e) => {
                eprintln!("Failed to scan panels: {}", e);
            }
        }
    }

    fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
    }

    fn attach_to_selected(&self) {
        if let Some(panel) = self.selected_panel() {
            // Create temp session and popup
            let temp_session = format!("pad-popup-{}", rand::random::<u32>());
            
            // This would spawn tmux commands
            // For now, just a placeholder
            let _ = Command::new("tmux")
                .args(&["new-session", "-d", "-s", &temp_session])
                .output();
        }
    }
}

fn capture_pane(pane_id: &str, lines: usize) -> Result<String, Box<dyn Error>> {
    let output = Command::new("tmux")
        .args(&["capture-pane", "-p", "-t", pane_id, "-S", &format!("-{}", lines)])
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("Failed to capture pane".into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Check for help argument
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        println!("pad - Tmux Code Kanban (Rust Edition)");
        println!("");
        println!("Usage: pad [OPTIONS]");
        println!("");
        println!("Options:");
        println!("  -h, --help     Show this help message");
        println!("  -V, --version  Show version");
        println!("");
        println!("Key bindings:");
        println!("  j/k or ↑/↓     Navigate panels");
        println!("  1-9            Jump to panel");
        println!("  Enter          Attach to panel");
        println!("  /              Search");
        println!("  r              Refresh");
        println!("  F1             Settings");
        println!("  q              Quit");
        return Ok(());
    }
    
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("pad 0.3.0");
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new();
    app.refresh_panels();
    
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.mode {
                        Mode::Normal => match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                app.should_quit = true;
                            }
                            KeyCode::Char('j') | KeyCode::Down => app.next(),
                            KeyCode::Char('k') | KeyCode::Up => app.previous(),
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                app.refresh_panels();
                            }
                            KeyCode::Char('/') => {
                                app.mode = Mode::Search;
                                app.is_searching = true;
                            }
                            KeyCode::Char('1') => app.table_state.select(Some(0)),
                            KeyCode::Char('2') => app.table_state.select(Some(1)),
                            KeyCode::Char('3') => app.table_state.select(Some(2)),
                            KeyCode::Char('4') => app.table_state.select(Some(3)),
                            KeyCode::Char('5') => app.table_state.select(Some(4)),
                            KeyCode::Char('6') => app.table_state.select(Some(5)),
                            KeyCode::Char('7') => app.table_state.select(Some(6)),
                            KeyCode::Char('8') => app.table_state.select(Some(7)),
                            KeyCode::Char('9') => app.table_state.select(Some(8)),
                            KeyCode::F(1) => app.toggle_settings(),
                            KeyCode::Enter => app.attach_to_selected(),
                            _ => {}
                        },
                        Mode::Search => match key.code {
                            KeyCode::Esc => {
                                app.mode = Mode::Normal;
                                app.is_searching = false;
                                app.search_query.clear();
                            }
                            KeyCode::Enter => {
                                app.mode = Mode::Normal;
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                            }
                            _ => {}
                        },
                        Mode::Settings => match key.code {
                            KeyCode::Esc | KeyCode::F(1) => {
                                app.mode = Mode::Normal;
                                app.settings_open = false;
                            }
                            _ => {}
                        },
                    }
                }
            }
        }

        // Auto refresh
        if last_tick.elapsed() >= tick_rate {
            if app.last_refresh.elapsed() >= app.refresh_interval {
                app.refresh_panels();
            }
            last_tick = Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
