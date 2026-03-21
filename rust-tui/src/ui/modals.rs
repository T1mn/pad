use crate::app::App;
use crate::theme::Theme;
use crate::tree::AgentLauncher;
use super::layout::centered_rect;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn draw_settings_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(50, 50, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" ⚙ Settings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width - 4,
        height: area.height - 2,
    };

    let items = app.settings_items();
    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(idx, (name, value, desc, editable))| {
            let is_selected = idx == app.settings_selected;

            let name_style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            let value_style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(if *editable { theme.accent } else { theme.comment })
            } else {
                Style::default().fg(if *editable { theme.accent } else { theme.comment })
            };

            let editable_marker = if *editable { " ›" } else { "" };

            Row::new(vec![
                Cell::from(*name).style(name_style),
                Cell::from(format!("{}{}", value, editable_marker)).style(value_style),
                Cell::from(*desc).style(Style::default().fg(theme.comment)),
            ])
        })
        .collect();

    let table = Table::new(rows, [
            Constraint::Length(18),
            Constraint::Length(15),
            Constraint::Min(0),
        ])
        .header(
            Row::new(vec!["Setting", "Value", "Description"])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        );

    f.render_widget(table, inner);
}

pub fn draw_theme_selector(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(40, 60, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" 🎨 Theme [{}] ", theme.name))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.keyword));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width - 4,
        height: area.height - 2,
    };

    let themes = App::available_themes();
    let rows: Vec<Row> = themes
        .iter()
        .enumerate()
        .map(|(idx, (name, desc))| {
            let is_selected = idx == app.theme_selected;
            let is_current = *name == app.config.theme;

            let marker = if is_current { "✓ " } else { "  " };

            let name_style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.keyword)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default()
                    .fg(theme.keyword)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            Row::new(vec![
                Cell::from(format!("{}{}", marker, name)).style(name_style),
                Cell::from(*desc).style(Style::default().fg(theme.comment)),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(20), Constraint::Min(0)])
        .header(
            Row::new(vec!["Theme", "Description"])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        );

    f.render_widget(table, inner);
}

pub fn draw_agent_launcher(f: &mut Frame, launcher: &AgentLauncher, area: Rect) {
    let popup_width = 50;
    let popup_height = 12;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(area.x + popup_x, area.y + popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let items: Vec<Row> = launcher
        .agents
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let prefix = if i == launcher.selected {
                "❯ "
            } else {
                "  "
            };
            let cells = vec![Cell::from(format!("{}{}", prefix, name))];
            let style = if i == launcher.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(cells).style(style)
        })
        .collect();

    let title = format!(" Select Agent for {} ", launcher.target_dir.display());
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let table = Table::new(items, [Constraint::Percentage(100)])
        .block(block);

    f.render_widget(table, popup_area);
}

pub fn draw_delete_confirm(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let popup_width = 50;
    let popup_height = 8;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(area.x + popup_x, area.y + popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let panel_info = if let Some(ref panel) = app.delete_target {
        format!("{}:{}.{}", panel.session, panel.window_index, panel.pane)
    } else {
        String::from("Unknown")
    };

    let text = format!(
        "Delete panel?\n\n{}\n\nPress 'y' to confirm\nAny other key to cancel",
        panel_info
    );

    let block = Block::default()
        .title(" ⚠️ Confirm Delete ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.error));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}

pub fn draw_help(f: &mut Frame, theme: &Theme, area: Rect) {
    let popup_area = centered_rect(60, 70, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" ? Help ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent));

    let help_lines = vec![
        Line::from(Span::styled(
            "pad - Tmux Agent Panel Manager",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / ↓         Move down"),
        Line::from("  k / ↑         Move up"),
        Line::from("  1-9           Jump to panel"),
        Line::from("  /             Search panels"),
        Line::from(""),
        Line::from(Span::styled(
            "Actions",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Enter         Attach to panel"),
        Line::from("  c             Create new session"),
        Line::from("  d             Delete panel"),
        Line::from("  r             Refresh panels"),
        Line::from("  PgUp/PgDn     Scroll preview"),
        Line::from("  Home/End      Preview top/bottom"),
        Line::from(""),
        Line::from(Span::styled(
            "File Tree",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  t             Toggle file tree"),
        Line::from("  T             Open tree at ~/"),
        Line::from("  Space         Expand/collapse dir"),
        Line::from("  Backspace     Go up directory"),
        Line::from("  J / K         Scroll file preview"),
        Line::from("  PgUp/PgDn     Scroll file preview"),
        Line::from(""),
        Line::from(Span::styled(
            "Other",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  F1            Settings"),
        Line::from("  ?             Toggle this help"),
        Line::from("  q             Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Detach from panel: F12 / Ctrl+Q / Ctrl+C",
            Style::default().fg(theme.comment),
        )),
    ];

    let paragraph = Paragraph::new(help_lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}
