use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn draw_panel_list(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let block = Block::default()
        .title(format!(" {} ", crate::i18n::t(l, "panel.title")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));

    // Adaptive columns based on width
    let show_git = area.width >= 80;
    let show_dir = area.width >= 60;

    let (rows, is_empty) = {
        let filtered = app.filtered_panels();
        let is_empty = filtered.is_empty();
        let rows: Vec<Row> = filtered
            .iter()
            .enumerate()
            .map(|(idx, panel)| {
                let index_str = if idx < 9 {
                    (idx + 1).to_string()
                } else {
                    String::new()
                };
                let status_style = match panel.state {
                    crate::model::AgentState::Busy => Style::default().fg(theme.warning),
                    crate::model::AgentState::Waiting => Style::default().fg(theme.success),
                    crate::model::AgentState::Idle => Style::default().fg(theme.comment),
                };

                let mut cells = vec![
                    Cell::from(index_str).style(Style::default().fg(theme.comment)),
                    Cell::from(panel.agent_type.emoji()),
                    Cell::from(panel.status_icon(app.busy_animation_frame)).style(status_style),
                    Cell::from(format!("{}:{}.{}", panel.session, panel.window, panel.pane)),
                ];

                if show_dir {
                    cells.push(
                        Cell::from(panel.shortened_path(20))
                            .style(Style::default().fg(theme.fg)),
                    );
                }
                if show_git {
                    cells.push(
                        Cell::from(panel.git_display()).style(Style::default().fg(theme.accent)),
                    );
                }

                Row::new(cells).height(1)
            })
            .collect();
        (rows, is_empty)
    };

    let mut widths: Vec<Constraint> = vec![
        Constraint::Length(2),  // Index
        Constraint::Length(4),  // Type
        Constraint::Length(3),  // Status
        Constraint::Length(20), // Location
    ];
    let mut headers: Vec<&str> = vec![
        crate::i18n::t(l, "table.num"),
        crate::i18n::t(l, "table.type"),
        crate::i18n::t(l, "table.status"),
        crate::i18n::t(l, "table.location"),
    ];

    if show_dir {
        widths.push(Constraint::Length(20));
        headers.push(crate::i18n::t(l, "table.dir"));
    }
    if show_git {
        widths.push(Constraint::Min(0));
        headers.push(crate::i18n::t(l, "table.git"));
    }

    let table = Table::new(rows, &widths)
        .header(
            Row::new(headers)
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(0),
        )
        .block(block)
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Always)
        .row_highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        );

    let mut table_state = app.table_state.clone();
    f.render_stateful_widget(table, area, &mut table_state);
    app.table_state = table_state;

    // Empty state
    if is_empty {
        let inner = Rect {
            x: area.x + 1,
            y: area.y + 2,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };
        let empty_msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_title"),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_hint"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_agents"),
                Style::default().fg(theme.accent),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_create"),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "panel.empty_refresh"),
                Style::default().fg(theme.fg),
            )),
        ];
        let empty = Paragraph::new(empty_msg)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(empty, inner);
    }
}

pub fn draw_file_tree(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(ref mut tree) = app.file_tree {
        let theme = &app.theme;
        tree.render(f, area, theme);
    } else {
        let l = app.locale;
        let block = Block::default()
            .title(format!(" {} ", crate::i18n::t(l, "tree.explorer")))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL);
        let paragraph = Paragraph::new(crate::i18n::t(l, "tree.no_dir"))
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }
}

pub fn draw_agent_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let l = app.locale;
    let active = app.panels.iter().filter(|p| p.is_active).count();
    let total = app.panels.len();
    let tmpl = crate::i18n::t(l, "panel.agent_count");
    let text = format!(" {} ",
        tmpl.replacen("{}", &total.to_string(), 1)
            .replacen("{}", &active.to_string(), 1)
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border));
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}
