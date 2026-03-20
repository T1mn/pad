use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal,
};
use std::io;

/// Fuzzy finder state
pub struct FuzzyPicker {
    /// All items to search
    items: Vec<String>,
    /// Filtered items with scores
    filtered: Vec<(String, u32)>,
    /// Current search query
    query: String,
    /// Selected index in filtered list
    selected: usize,
    /// Whether the picker is active
    active: bool,
}

impl FuzzyPicker {
    pub fn new(items: Vec<String>) -> Self {
        let filtered: Vec<_> = items.iter().map(|s| (s.clone(), 0)).collect();
        Self {
            items,
            filtered,
            query: String::new(),
            selected: 0,
            active: true,
        }
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Update filter based on current query
    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = self.items.iter().map(|s| (s.clone(), 0)).collect();
        } else {
            let mut matcher = Matcher::default();
            let pattern = Pattern::parse(&self.query, CaseMatching::Smart, Normalization::Smart);
            
            let mut buf = Vec::new();
            let mut results: Vec<(String, u32)> = self
                .items
                .iter()
                .filter_map(|item| {
                    buf.clear();
                    // Convert item to utf32 string for matching
                    let utf32_str = Utf32Str::new(item, &mut buf);
                    
                    pattern.score(utf32_str, &mut matcher)
                        .map(|score| (item.clone(), score))
                })
                .collect();
            
            // Sort by score (descending)
            results.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = results;
        }
        
        // Reset selection if out of bounds
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn handle_input(&mut self, key: crossterm::event::KeyEvent) -> Option<Option<String>> {
        use crossterm::event::{KeyCode, KeyEventKind};
        
        if key.kind != KeyEventKind::Press {
            return None;
        }
        
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                Some(None) // Cancelled
            }
            KeyCode::Enter => {
                self.active = false;
                if let Some((item, _)) = self.filtered.get(self.selected) {
                    Some(Some(item.clone()))
                } else {
                    Some(None)
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
                None
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.update_filter();
                None
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.update_filter();
                None
            }
            _ => None,
        }
    }

    pub fn draw(&self, f: &mut ratatui::Frame) {
        let area = centered_rect(70, 70, f.size());
        
        // Clear background
        f.render_widget(Clear, area);
        
        // Main block
        let block = Block::default()
            .title(" Select Directory ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        
        let inner = area.inner(&Margin::new(2, 1));
        
        // Split into query area and list area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(inner);
        
        // Query input
        let query_block = Block::default()
            .title(" Filter ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        
        let query_text = Paragraph::new(self.query.clone())
            .block(query_block)
            .wrap(Wrap { trim: false });
        
        f.render_widget(query_text, chunks[0]);
        
        // List of items
        let list_height = chunks[1].height as usize;
        let start_idx = if self.selected > list_height / 2 {
            (self.selected - list_height / 2).min(self.filtered.len().saturating_sub(list_height))
        } else {
            0
        };
        let end_idx = (start_idx + list_height).min(self.filtered.len());
        
        let visible_items: Vec<Line> = self.filtered[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(idx, (item, _score))| {
                let actual_idx = start_idx + idx;
                let style = if actual_idx == self.selected {
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                
                let prefix = if actual_idx == self.selected { "❯ " } else { "  " };
                Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(item.clone(), style),
                ])
            })
            .collect();
        
        if visible_items.is_empty() {
            let empty_text = Paragraph::new("No matches")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty_text, chunks[1]);
        } else {
            // Convert Vec<Line> to Text
            let text = ratatui::text::Text::from(visible_items);
            let list_text = Paragraph::new(text);
            f.render_widget(list_text, chunks[1]);
        }
        
        // Render border last
        f.render_widget(block, area);
        
        // Help text at bottom
        let help = Paragraph::new("↑/k ↓/j: navigate | type: filter | Enter: select | Esc: cancel")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: area.width,
            height: 1,
        };
        f.render_widget(help, help_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Scan directories and show fuzzy picker
pub fn fuzzy_select_directory() -> io::Result<Option<String>> {
    // Get home directory
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    
    // Scan directories (limit depth to avoid too many items)
    let items = scan_directories(&home, 3);
    
    if items.is_empty() {
        return Ok(Some(home));
    }
    
    // Setup terminal for fuzzy picker
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let mut picker = FuzzyPicker::new(items);
    let result = loop {
        terminal.draw(|f| {
            picker.draw(f);
        })?;
        
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match picker.handle_input(key) {
                    None => continue,
                    Some(result) => break Some(result),
                }
            }
        }
    };
    
    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    // result is Option<Option<String>> (outer Some means loop completed)
    Ok(result.flatten())
}

/// Scan directories up to max_depth
fn scan_directories(base: &str, max_depth: usize) -> Vec<String> {
    let mut results = vec![base.to_string()];
    
    if max_depth == 0 {
        return results;
    }
    
    let base_path = std::path::Path::new(base);
    if let Ok(entries) = std::fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let path_str = path.to_string_lossy().to_string();
                
                // Skip hidden directories
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }
                
                results.push(path_str.clone());
                
                // Recursively scan (limit depth)
                if max_depth > 1 {
                    let sub_dirs = scan_directories(&path_str, max_depth - 1);
                    results.extend(sub_dirs.into_iter().skip(1)); // Skip duplicate base
                }
            }
        }
    }
    
    // Sort and remove duplicates
    results.sort();
    results.dedup();
    results
}

// Add dirs crate dependency helper
pub mod dirs {
    pub fn home_dir() -> Option<std::path::PathBuf> {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .ok()
    }
}
