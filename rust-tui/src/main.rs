use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use ratatui::{
    backend::CrosstermBackend,
    widgets::TableState,
    Terminal,
};
use std::{
    collections::HashMap,
    error::Error,
    io::{self, Read, Write},
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

mod fuzzy;
mod model;
mod scanner;
mod tree;
mod ui;

use fuzzy::fuzzy_select_directory;
use model::CodePanel;
use scanner::scan_panels;

/// Settings configuration
#[derive(Clone, Debug)]
pub struct Settings {
    pub theme: String,
    pub auto_refresh: bool,
    pub refresh_interval: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: String::from("default"),
            auto_refresh: true,
            refresh_interval: Duration::from_secs(10),
        }
    }
}

/// Async scan result channel type
type ScanResult = Result<Vec<CodePanel>, Box<dyn Error + Send + Sync>>;

/// Application state
struct App {
    panels: Vec<CodePanel>,
    table_state: TableState,
    mode: Mode,
    last_refresh: Instant,
    #[allow(dead_code)]
    refresh_interval: Duration,
    search_query: String,
    is_searching: bool,
    preview_content: String,
    preview_pane_id: Option<String>,
    #[allow(dead_code)]
    content_hashes: HashMap<String, String>,
    settings_open: bool,
    settings: Settings,
    theme_selector_open: bool,
    settings_selected: usize,
    theme_selected: usize,
    scan_in_progress: bool,
    scan_rx: Option<mpsc::Receiver<ScanResult>>,
    preview_update_in_progress: bool,
    preview_rx: Option<mpsc::Receiver<(String, String)>>,
    last_preview_update: Instant,
    refresh_after_attach: bool,
    should_quit: bool,
    dirty: bool,
    show_tree: bool,
    file_tree: Option<tree::FileTree>,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Search,
    Settings,
    ThemeSelector,
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
            refresh_interval: Duration::from_secs(10),
            search_query: String::new(),
            is_searching: false,
            preview_content: String::from("Select a panel to preview"),
            preview_pane_id: None,
            content_hashes: HashMap::new(),
            settings_open: false,
            settings: Settings::default(),
            theme_selector_open: false,
            settings_selected: 0,
            theme_selected: 0,
            scan_in_progress: false,
            scan_rx: None,
            preview_update_in_progress: false,
            preview_rx: None,
            last_preview_update: Instant::now(),
            refresh_after_attach: false,
            should_quit: false,
            dirty: true,
            show_tree: false,
            file_tree: None,
        }
    }

    fn next(&mut self) {
        if self.show_tree {
            if let Some(ref mut tree) = self.file_tree {
                tree.select_next();
                self.dirty = true;
                return;
            }
        }
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
        self.preview_pane_id = None;
        self.update_tree_for_selection();
        self.dirty = true;
    }

    fn previous(&mut self) {
        if self.show_tree {
            if let Some(ref mut tree) = self.file_tree {
                tree.select_previous();
                self.dirty = true;
                return;
            }
        }
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
        self.preview_pane_id = None;
        self.update_tree_for_selection();
        self.dirty = true;
    }

    fn jump_to(&mut self, index: usize) {
        if index < self.filtered_panels().len() {
            self.table_state.select(Some(index));
            self.preview_pane_id = None;
            self.dirty = true;
        }
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

    fn toggle_tree(&mut self) {
        self.show_tree = !self.show_tree;
        if self.show_tree {
            // Initialize tree for selected panel's working directory
            if let Some(panel) = self.selected_panel() {
                let path = PathBuf::from(&panel.working_dir);
                if path.exists() {
                    self.file_tree = Some(tree::FileTree::new(path));
                }
            }
        } else {
            self.file_tree = None;
        }
        self.dirty = true;
    }

    fn update_tree_for_selection(&mut self) {
        if self.show_tree {
            if let Some(panel) = self.selected_panel() {
                let path = PathBuf::from(&panel.working_dir);
                if path.exists() {
                    // Only update if path changed
                    let should_update = match &self.file_tree {
                        None => true,
                        Some(tree) => tree.root_path != path,
                    };
                    if should_update {
                        self.file_tree = Some(tree::FileTree::new(path));
                        self.dirty = true;
                    }
                }
            }
        }
    }

    fn trigger_async_scan(&mut self) {
        if self.scan_in_progress {
            return;
        }
        
        self.scan_in_progress = true;
        let (tx, rx) = mpsc::channel::<ScanResult>(1);
        self.scan_rx = Some(rx);
        
        tokio::task::spawn_blocking(move || {
            let result = scan_panels();
            let _ = tx.blocking_send(result);
        });
    }

    fn check_scan_result(&mut self) {
        if let Some(ref mut rx) = self.scan_rx {
            match rx.try_recv() {
                Ok(Ok(panels)) => {
                    self.panels = panels;
                    self.last_refresh = Instant::now();
                    self.preview_pane_id = None;
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                    self.dirty = true;
                }
                Ok(Err(_)) => {
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                }
            }
        }
    }

    fn trigger_async_preview_update(&mut self, pane_id: String) {
        if self.preview_update_in_progress {
            return;
        }
        
        self.preview_update_in_progress = true;
        let (tx, rx) = mpsc::channel::<(String, String)>(1);
        self.preview_rx = Some(rx);
        
        tokio::task::spawn_blocking(move || {
            let content = match capture_pane(&pane_id, 50) {
                Ok(content) => content,
                Err(_) => String::from("Failed to capture pane"),
            };
            let _ = tx.blocking_send((pane_id, content));
        });
    }

    fn check_preview_result(&mut self) {
        if let Some(ref mut rx) = self.preview_rx {
            match rx.try_recv() {
                Ok((pane_id, content)) => {
                    self.preview_content = content;
                    self.preview_pane_id = Some(pane_id);
                    self.preview_update_in_progress = false;
                    self.preview_rx = None;
                    self.last_preview_update = Instant::now();
                    self.dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.preview_update_in_progress = false;
                    self.preview_rx = None;
                }
            }
        }
    }

    fn check_preview_update(&mut self) {
        // Debounce: wait at least 500ms between preview updates
        if self.last_preview_update.elapsed() < Duration::from_millis(500) {
            return;
        }
        
        // Skip if already updating or scanning
        if self.preview_update_in_progress || self.scan_in_progress {
            return;
        }
        
        let pane_id = self.selected_panel().map(|p| p.pane_id.clone());
        
        if let Some(pane_id) = pane_id {
            let needs_update = match &self.preview_pane_id {
                None => true,
                Some(id) if id != &pane_id => true,
                _ => false,
            };

            if needs_update {
                self.trigger_async_preview_update(pane_id);
            }
        }
    }

    fn refresh_panels(&mut self) {
        if !self.scan_in_progress {
            self.trigger_async_scan();
        }
    }

    fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
        if self.settings_open {
            self.mode = Mode::Settings;
            self.settings_selected = 0;
        } else {
            self.mode = Mode::Normal;
        }
        self.dirty = true;
    }

    fn open_theme_selector(&mut self) {
        self.theme_selector_open = true;
        self.mode = Mode::ThemeSelector;
        self.theme_selected = 0;
        self.dirty = true;
    }

    fn close_theme_selector(&mut self) {
        self.theme_selector_open = false;
        self.mode = Mode::Settings;
        self.dirty = true;
    }

    fn settings_items(&self) -> Vec<(&str, String, &str, bool)> {
        vec![
            ("Theme", self.settings.theme.clone(), "Color scheme", true),
            (
                "Auto Refresh",
                if self.settings.auto_refresh {
                    "On".to_string()
                } else {
                    "Off".to_string()
                },
                "Auto-refresh panel list",
                true,
            ),
            (
                "Refresh Interval",
                format!("{}s", self.settings.refresh_interval.as_secs()),
                "Seconds between panel list refreshes",
                false,
            ),
            ("Version", "0.4.0".to_string(), "tmux-code-kanban (Rust)", false),
        ]
    }

    fn available_themes() -> Vec<(&'static str, &'static str)> {
        vec![
            ("default", "Default"),
            ("dark", "Dark"),
            ("dracula", "Dracula"),
            ("nord", "Nord"),
            ("gruvbox", "Gruvbox"),
            ("catppuccin", "Catppuccin"),
            ("tokyo-night", "Tokyo Night"),
            ("monokai", "Monokai"),
            ("solarized-dark", "Solarized Dark"),
            ("rose-pine", "Rose Pine"),
        ]
    }
}

/// Create a new tmux session using native fuzzy finder to select path
fn create_new_session_fuzzy() -> Result<(), Box<dyn Error>> {
    // Use native fuzzy picker
    match fuzzy_select_directory() {
        Ok(Some(selected)) => {
            create_session_in_path(&selected)?;
        }
        Ok(None) => {
            // User cancelled
        }
        Err(e) => {
            return Err(format!("Failed to show directory picker: {}", e).into());
        }
    }
    
    Ok(())
}

/// Create a new tmux session in the given path
fn create_session_in_path(path: &str) -> Result<(), Box<dyn Error>> {
    // Generate session name from path
    let session_name = path
        .replace("/", "_")
        .replace(".", "_")
        .replace("~", "home");
    
    // Check if session already exists
    let check = Command::new("tmux")
        .args(&["has-session", "-t", &session_name])
        .output()?;
    
    if check.status.success() {
        // Session exists, just switch to it
        println!("Session '{}' already exists, attaching...", session_name);
        Command::new("tmux")
            .args(&["switch-client", "-t", &session_name])
            .spawn()?;
    } else {
        // Create new session
        println!("Creating new session '{}' in {}...", session_name, path);
        Command::new("tmux")
            .args(&["new-session", "-d", "-s", &session_name, "-c", path])
            .output()?;
    }
    
    Ok(())
}

/// Attach to a tmux pane using PTY (direct attach, not popup)
fn attach_to_pane_pty(panel: &CodePanel) -> Result<(), Box<dyn Error>> {
    // Use window_index for reliable targeting (not window name which can be dynamic)
    let target = format!("{}:{}", panel.session, panel.window_index);
    
    let pty_system = NativePtySystem::default();
    
    let (cols, rows) = crossterm::terminal::size()
        .map(|(w, h)| (w as u16, h as u16))
        .unwrap_or((80, 24));
    
    let pty = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    
    let mut cmd = CommandBuilder::new("tmux");
    cmd.arg("attach-session");
    cmd.arg("-t");
    cmd.arg(&target);
    
    let mut child = pty.slave.spawn_command(cmd)?;
    
    let mut master_reader = pty.master.try_clone_reader()?;
    let mut master_writer = pty.master.take_writer()?;
    
    // Set raw mode
    let _guard = set_raw_mode()?;
    
    // Spawn thread to copy PTY output to stdout
    std::thread::scope(|s| {
        s.spawn(|| {
            let mut buf = [0u8; 1024];
            let mut stdout = io::stdout();
            loop {
                match master_reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if stdout.write_all(&buf[..n]).is_err() {
                            break;
                        }
                        let _ = stdout.flush();
                    }
                    Err(_) => break,
                }
            }
        });
        
        // Main thread: Read stdin, intercept F12 (ESC[24~) and Ctrl+Q
        let mut stdin = io::stdin();
        let mut buf = [0u8; 64];
        let mut pending = Vec::new(); // Buffer for escape sequence detection
        
        // Helper function to check if bytes contain a complete exit sequence
        fn find_exit_sequence(pending: &[u8]) -> Option<(usize, &'static str)> {
            // Check for F12: ESC[24~
            if let Some(pos) = pending.windows(5).position(|w| w == &[0x1b, b'[', b'2', b'4', b'~']) {
                return Some((pos, "F12"));
            }
            // Check for F12 variant with modifier: ESC[24;*~
            if pending.len() >= 6 {
                if let Some(pos) = pending.windows(4).position(|w| w == &[0x1b, b'[', b'2', b'4']) {
                    if pending.len() > pos + 4 && pending[pos + 4] == b';' {
                        // Look for closing ~
                        if let Some(_end) = pending[pos + 5..].iter().position(|&b| b == b'~') {
                            return Some((pos, "F12-modified"));
                        }
                    }
                }
            }
            // Check for Ctrl+Q (0x11) or Ctrl+C (0x03)
            if let Some(pos) = pending.iter().position(|&b| b == 0x11 || b == 0x03) {
                return Some((pos, if pending[pos] == 0x11 { "Ctrl+Q" } else { "Ctrl+C" }));
            }
            None
        }
        
        // Helper to find safe bytes to forward (not part of potential escape sequence)
        fn find_safe_forward_point(pending: &[u8]) -> usize {
            // Find last position that is definitely not part of an escape sequence
            // Escape sequences start with ESC (0x1b) or CSI (0x9b)
            if let Some(esc_pos) = pending.iter().rposition(|&b| b == 0x1b) {
                // If we have ESC, only forward up to before it
                // Unless there's enough data to determine it's not a sequence we care about
                if esc_pos > 0 {
                    return esc_pos;
                }
                return 0; // Don't forward anything, wait for more data
            }
            pending.len()
        }
        
        loop {
            match stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    pending.extend_from_slice(&buf[..n]);
                    
                    // Check for exit sequences first
                    if let Some((pos, key)) = find_exit_sequence(&pending) {
                        // Forward bytes before the exit key
                        if pos > 0 {
                            let _ = master_writer.write_all(&pending[..pos]);
                            let _ = master_writer.flush();
                        }
                        // Send the exit key to tmux first (so it can process it)
                        let _ = master_writer.write_all(&pending[pos..pos + if key.starts_with("F12") { 5 } else { 1 }]);
                        let _ = master_writer.flush();
                        std::thread::sleep(Duration::from_millis(50)); // Brief delay for tmux
                        break;
                    }
                    
                    // Forward safe bytes (not potential escape sequences)
                    let safe_len = find_safe_forward_point(&pending);
                    if safe_len > 0 {
                        if master_writer.write_all(&pending[..safe_len]).is_err() {
                            break;
                        }
                        let _ = master_writer.flush();
                        pending.drain(..safe_len);
                    }
                    
                    // Prevent buffer overflow (shouldn't happen with normal input)
                    if pending.len() > 100 {
                        // Flush everything to avoid getting stuck
                        let _ = master_writer.write_all(&pending);
                        let _ = master_writer.flush();
                        pending.clear();
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    let _ = child.kill();
    
    // Reset terminal style
    print!("\x1b[0m\x1b[24m\x1b[39m\x1b[49m");
    let _ = io::stdout().flush();
    
    Ok(())
}

#[cfg(unix)]
fn set_raw_mode() -> Result<RawModeGuard, Box<dyn Error>> {
    use std::os::fd::AsRawFd;
    
    let stdin = io::stdin();
    let fd = stdin.as_raw_fd();
    
    // Convert RawFd to BorrowedFd for nix 0.27+ API
    let borrowed_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(fd) };
    
    let termios = nix::sys::termios::tcgetattr(borrowed_fd)?;
    let original = termios.clone();
    
    let mut raw = termios;
    nix::sys::termios::cfmakeraw(&mut raw);
    nix::sys::termios::tcsetattr(borrowed_fd, nix::sys::termios::SetArg::TCSAFLUSH, &raw)?;
    
    Ok(RawModeGuard { fd, original })
}

#[cfg(not(unix))]
fn set_raw_mode() -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[cfg(unix)]
struct RawModeGuard {
    fd: std::os::fd::RawFd,
    original: nix::sys::termios::Termios,
}

#[cfg(unix)]
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let borrowed_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(self.fd) };
        let _ = nix::sys::termios::tcsetattr(
            borrowed_fd,
            nix::sys::termios::SetArg::TCSAFLUSH,
            &self.original,
        );
    }
}

#[cfg(not(unix))]
struct RawModeGuard;

#[cfg(not(unix))]
impl Drop for RawModeGuard {
    fn drop(&mut self) {}
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
        println!("  j/k or ↑/↓     Navigate panels / tree");
        println!("  1-9            Jump to panel");
        println!("  Enter          Attach to panel (F12 or Ctrl+Q to detach)");
        println!("  t              Toggle file tree explorer");
        println!("  Space          Expand/collapse directory (in tree view)");
        println!("  /              Search");
        println!("  r              Refresh");
        println!("  c              Create new session");
        println!("  F1             Settings");
        println!("  q              Quit");
        return Ok(());
    }
    
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("pad 0.4.0");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    if let Ok(panels) = scan_panels() {
        app.panels = panels;
    }
    
    let res = run_app(&mut terminal, &mut app).await;

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

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(16);
    let mut last_preview_refresh = Instant::now();

    loop {
        if app.refresh_after_attach {
            app.refresh_after_attach = false;
            app.refresh_panels();
            app.preview_pane_id = None;
        }
        
        app.check_scan_result();
        app.check_preview_result();

        if last_preview_refresh.elapsed() >= Duration::from_millis(500) {
            app.check_preview_update();
            last_preview_refresh = Instant::now();
        }

        // Only draw when there are changes
        if app.dirty {
            terminal.draw(|f| ui::draw(f, app))?;
            app.dirty = false;
        }

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
                                app.dirty = true;
                            }
                            KeyCode::Char('/') => {
                                app.mode = Mode::Search;
                                app.is_searching = true;
                                app.dirty = true;
                            }
                            KeyCode::Char('1') => app.jump_to(0),
                            KeyCode::Char('2') => app.jump_to(1),
                            KeyCode::Char('3') => app.jump_to(2),
                            KeyCode::Char('4') => app.jump_to(3),
                            KeyCode::Char('5') => app.jump_to(4),
                            KeyCode::Char('6') => app.jump_to(5),
                            KeyCode::Char('7') => app.jump_to(6),
                            KeyCode::Char('8') => app.jump_to(7),
                            KeyCode::Char('9') => app.jump_to(8),
                            KeyCode::F(1) => {
                                app.toggle_settings();
                                app.dirty = true;
                            }
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                app.toggle_tree();
                            }
                            KeyCode::Char(' ') => {
                                if app.show_tree {
                                    if let Some(ref mut tree) = app.file_tree {
                                        tree.toggle_selected();
                                    }
                                    app.dirty = true;
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(panel) = app.selected_panel() {
                                    let panel = panel.clone();
                                    
                                    // Restore terminal
                                    disable_raw_mode()?;
                                    execute!(
                                        terminal.backend_mut(),
                                        LeaveAlternateScreen,
                                        DisableMouseCapture
                                    )?;
                                    terminal.show_cursor()?;
                                    
                                    // Show hint in normal screen (not alternate)
                                    print!("\x1b[2J\x1b[H"); // Clear screen
                                    println!("Attaching to {} @ {} (window {})", 
                                        panel.code_type, panel.pane_id, panel.window_index);
                                    println!("Press F12 or Ctrl+Q to return to pad (or Ctrl+C as emergency exit)\n");
                                    io::stdout().flush()?;
                                    
                                    // Small delay to ensure message is visible
                                    std::thread::sleep(Duration::from_millis(100));
                                    
                                    // PTY attach
                                    if let Err(e) = attach_to_pane_pty(&panel) {
                                        println!("Attach error: {}", e);
                                        println!("Press any key to continue...");
                                        io::stdout().flush()?;
                                        // Wait for key press
                                        let _ = crossterm::event::read();
                                    }
                                    
                                    // Clear screen before returning to TUI
                                    print!("\x1b[2J\x1b[H");
                                    io::stdout().flush()?;
                                    
                                    // Re-setup terminal
                                    enable_raw_mode()?;
                                    execute!(
                                        terminal.backend_mut(),
                                        EnterAlternateScreen,
                                        EnableMouseCapture
                                    )?;
                                    
                                    // Clear the terminal to ensure clean state
                                    terminal.clear()?;
                                    
                                    app.refresh_after_attach = true;
                                    app.dirty = true;
                                }
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                // Restore terminal for fzf
                                disable_raw_mode()?;
                                execute!(
                                    terminal.backend_mut(),
                                    LeaveAlternateScreen,
                                    DisableMouseCapture
                                )?;
                                terminal.show_cursor()?;
                                
                                // Clear screen
                                print!("\x1b[2J\x1b[H");
                                io::stdout().flush()?;
                                
                                // Run fzf path selection
                                if let Err(e) = create_new_session_fuzzy() {
                                    println!("Error: {}", e);
                                    println!("\nPress any key to continue...");
                                    let _ = crossterm::event::read();
                                }
                                
                                // Clear screen before returning
                                print!("\x1b[2J\x1b[H");
                                io::stdout().flush()?;
                                
                                // Re-setup terminal
                                enable_raw_mode()?;
                                execute!(
                                    terminal.backend_mut(),
                                    EnterAlternateScreen,
                                    EnableMouseCapture
                                )?;
                                
                                // Refresh panels to show new session
                                app.refresh_panels();
                            }
                            _ => {}
                        },
                        Mode::Search => match key.code {
                            KeyCode::Esc => {
                                app.mode = Mode::Normal;
                                app.is_searching = false;
                                app.search_query.clear();
                                app.dirty = true;
                            }
                            KeyCode::Enter => {
                                app.mode = Mode::Normal;
                                app.dirty = true;
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.preview_pane_id = None;
                                app.dirty = true;
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.preview_pane_id = None;
                                app.dirty = true;
                            }
                            _ => {}
                        },
                        Mode::Settings => match key.code {
                            KeyCode::Esc | KeyCode::F(1) => {
                                app.settings_open = false;
                                app.mode = Mode::Normal;
                                app.dirty = true;
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                let max = app.settings_items().len().saturating_sub(1);
                                if app.settings_selected < max {
                                    app.settings_selected += 1;
                                }
                                app.dirty = true;
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                if app.settings_selected > 0 {
                                    app.settings_selected -= 1;
                                }
                                app.dirty = true;
                            }
                            KeyCode::Char('1') => { app.settings_selected = 0; app.dirty = true; }
                            KeyCode::Char('2') => { app.settings_selected = 1.min(app.settings_items().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Char('3') => { app.settings_selected = 2.min(app.settings_items().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Char('4') => { app.settings_selected = 3.min(app.settings_items().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Enter => {
                                let items = app.settings_items();
                                if let Some((name, _, _, editable)) = items.get(app.settings_selected) {
                                    if *editable {
                                        match *name {
                                            "Theme" => app.open_theme_selector(),
                                            "Auto Refresh" => {
                                                app.settings.auto_refresh = !app.settings.auto_refresh;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                app.dirty = true;
                            }
                            _ => {}
                        },
                        Mode::ThemeSelector => match key.code {
                            KeyCode::Esc => {
                                app.close_theme_selector();
                                app.dirty = true;
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                let max = App::available_themes().len().saturating_sub(1);
                                if app.theme_selected < max {
                                    app.theme_selected += 1;
                                }
                                app.dirty = true;
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                if app.theme_selected > 0 {
                                    app.theme_selected -= 1;
                                }
                                app.dirty = true;
                            }
                            KeyCode::Char('1') => { app.theme_selected = 0; app.dirty = true; }
                            KeyCode::Char('2') => { app.theme_selected = 1.min(App::available_themes().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Char('3') => { app.theme_selected = 2.min(App::available_themes().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Char('4') => { app.theme_selected = 3.min(App::available_themes().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Char('5') => { app.theme_selected = 4.min(App::available_themes().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Char('6') => { app.theme_selected = 5.min(App::available_themes().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Char('7') => { app.theme_selected = 6.min(App::available_themes().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Char('8') => { app.theme_selected = 7.min(App::available_themes().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Char('9') => { app.theme_selected = 8.min(App::available_themes().len().saturating_sub(1)); app.dirty = true; }
                            KeyCode::Enter => {
                                let themes = App::available_themes();
                                if let Some((name, _)) = themes.get(app.theme_selected) {
                                    app.settings.theme = name.to_string();
                                    app.close_theme_selector();
                                }
                                app.dirty = true;
                            }
                            _ => {}
                        },
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            if app.settings.auto_refresh && app.last_refresh.elapsed() >= app.settings.refresh_interval {
                if !app.scan_in_progress {
                    app.trigger_async_scan();
                }
            }
            last_tick = Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
