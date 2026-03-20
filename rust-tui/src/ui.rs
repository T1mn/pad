use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::{App, Mode};

pub fn draw(f: &mut Frame, app: &App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

    let body_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_layout[0]);

    // Left panel - list
    draw_panel_list(f, app, body_layout[0]);

    // Right panel - preview
    draw_preview(f, app, body_layout[1]);

    // Bottom status bar
    draw_status_bar(f, app, main_layout[1]);

    // Settings modal (overlay)
    if app.settings_open {
        draw_settings_modal(f, app);
    }

    // Theme selector modal (overlay on top of settings)
    if app.theme_selector_open {
        draw_theme_selector(f, app);
    }
}

fn draw_panel_list(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Code Panels ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    let filtered = app.filtered_panels();
    
    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(idx, panel)| {
            let index_str = if idx < 9 {
                (idx + 1).to_string()
            } else {
                String::new()
            };

            let cells = vec![
                Cell::from(index_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(panel.code_type.emoji()),
                Cell::from(panel.status_icon()),
                Cell::from(format!("{}:{}.{}", panel.session, panel.window, panel.pane)),
                Cell::from(panel.shortened_path(20)).style(Style::default().fg(Color::Gray)),
                Cell::from(panel.git_display()).style(Style::default().fg(Color::Cyan)),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let table = Table::new(rows)
        .widths(&[
            Constraint::Length(2),  // Index
            Constraint::Length(4),  // Type
            Constraint::Length(3),  // Status
            Constraint::Length(20), // Location
            Constraint::Length(20), // Directory
            Constraint::Min(0),     // Git
        ])
    .header(
        Row::new(vec!["#", "Type", "St", "Location", "Directory", "Git"])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(0)
    )
    .block(block)
    .highlight_spacing(ratatui::widgets::HighlightSpacing::Always)
    .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    let mut table_state = app.table_state.clone();
    f.render_stateful_widget(table, area, &mut table_state);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    // Get selected panel info for title
    let title = if let Some(panel) = app.selected_panel() {
        let git_info = if let Some(git) = &panel.git_info {
            if let Some(branch) = &git.branch {
                format!(" [{}]", branch)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        format!(" Preview: {}{}{} ", panel.pane_id, git_info, " ".repeat(100))
    } else {
        String::from(" Preview ")
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    // Process preview content with syntax highlighting
    let lines: Vec<Line> = app
        .preview_content
        .lines()
        .map(|line| Line::from(format_line(line)))
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((0, 0));

    f.render_widget(paragraph, area);
}

fn format_line(line: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let stripped = line.trim();

    // User prompts
    let user_markers = ["$", "#", "❯", ">", "%"];
    for marker in &user_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            spans.push(Span::styled(
                *marker,
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {}", content),
                Style::default().fg(Color::Green),
            ));
            return spans;
        }
    }

    // AI markers
    let ai_markers = ["●", "•", "💫", "🤖", "🟣", "🔵", "🟢", "⚡"];
    for marker in &ai_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            spans.push(Span::styled(
                *marker,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {}", content),
                Style::default().fg(Color::Cyan),
            ));
            return spans;
        }
    }

    // Error messages
    if stripped.to_lowercase().contains("error")
        || stripped.to_lowercase().contains("failed")
    {
        spans.push(Span::styled(line, Style::default().fg(Color::Red)));
        return spans;
    }

    // Success messages
    if stripped.to_lowercase().contains("success")
        || stripped.to_lowercase().contains("done")
        || stripped.contains("✓")
    {
        spans.push(Span::styled(line, Style::default().fg(Color::Green)));
        return spans;
    }

    // Default
    spans.push(Span::raw(line));
    spans
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (msg, style) = match app.mode {
        Mode::Search => (
            format!("SEARCH: {} | Enter: confirm | Esc: cancel", app.search_query),
            Style::default().fg(Color::Yellow),
        ),
        Mode::Settings => (
            String::from("j/k: move | Enter: edit | 1-4: jump | Esc: close"),
            Style::default().fg(Color::Cyan),
        ),
        Mode::ThemeSelector => (
            String::from("j/k: move | Enter: select | 1-9: jump | Esc: back"),
            Style::default().fg(Color::Cyan),
        ),
        _ => {
            let panel_count = app.filtered_panels().len();
            let status = format!(
                "↑/k ↓/j | 1-9 jmp | / find | ⏎ attach | c create | r refresh | F1 settings | q quit | {} panels",
                panel_count
            );
            (status, Style::default().fg(Color::White))
        }
    };

    let status_bar = Paragraph::new(msg)
        .style(style)
        .alignment(Alignment::Left);

    f.render_widget(status_bar, area);
}

fn draw_settings_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, f.size());

    // Clear background
    f.render_widget(Clear, area);

    // Draw border block
    let block = Block::default()
        .title(" ⚙ Settings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    f.render_widget(block, area);

    // Inner area for content
    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width - 4,
        height: area.height - 2,
    };

    // Build settings rows
    let items = app.settings_items();
    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(idx, (name, value, desc, editable))| {
            let is_selected = idx == app.settings_selected;
            
            let name_style = if is_selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            let value_style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(if *editable { Color::Cyan } else { Color::Gray })
            } else {
                Style::default().fg(if *editable { Color::Cyan } else { Color::Gray })
            };

            let editable_marker = if *editable { " ›" } else { "" };

            Row::new(vec![
                Cell::from(*name).style(name_style),
                Cell::from(format!("{}{}", value, editable_marker)).style(value_style),
                Cell::from(*desc).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let table = Table::new(rows)
        .widths(&[
            Constraint::Length(18),
            Constraint::Length(15),
            Constraint::Min(0),
        ])
        .header(
            Row::new(vec!["Setting", "Value", "Description"])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1)
        );

    f.render_widget(table, inner);
}

fn draw_theme_selector(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 60, f.size());

    // Clear background
    f.render_widget(Clear, area);

    // Draw border block
    let block = Block::default()
        .title(" 🎨 Theme ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    f.render_widget(block, area);

    // Inner area for content
    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width - 4,
        height: area.height - 2,
    };

    // Build theme rows
    let themes = App::available_themes();
    let rows: Vec<Row> = themes
        .iter()
        .enumerate()
        .map(|(idx, (name, desc))| {
            let is_selected = idx == app.theme_selected;
            let is_current = *name == app.settings.theme;
            
            let marker = if is_current { "✓ " } else { "  " };
            
            let name_style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!("{}{}", marker, name)).style(name_style),
                Cell::from(*desc).style(Style::default().fg(Color::Gray)),
            ])
        })
        .collect();

    let table = Table::new(rows)
        .widths(&[Constraint::Length(20), Constraint::Min(0)])
        .header(
            Row::new(vec!["Theme", "Description"])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1)
        );

    f.render_widget(table, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
