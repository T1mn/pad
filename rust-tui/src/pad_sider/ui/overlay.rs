use super::super::{app::App, preview::MarkdownPreview, search::FileSearch};
use super::line_numbers::add_line_numbers;
use super::markdown::render_markdown;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_preview(frame: &mut Frame, app: &App, preview: &MarkdownPreview) {
    let title = format!(
        " preview {} ",
        preview
            .path
            .strip_prefix(&app.cwd)
            .unwrap_or(&preview.path)
            .display()
    );
    let paragraph = Paragraph::new(add_line_numbers(render_markdown(&preview.content)))
        .block(focus_block(&title, true))
        .wrap(Wrap { trim: false })
        .scroll((preview.scroll, 0));
    frame.render_widget(paragraph, frame.area());
}

pub fn draw_search(frame: &mut Frame, search: &FileSearch) {
    let area = centered_rect(78, 70, frame.area());
    let inner = area.inner(Margin::new(2, 1));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .title(" search files ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
        area,
    );

    let query = Paragraph::new(search.query().to_string())
        .block(Block::default().title(" / ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(query, chunks[0]);

    if search.len() == 0 {
        let empty = Paragraph::new("No matches")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let list_height = chunks[1].height as usize;
    let selected = search.selected();
    let start = if selected > list_height / 2 {
        (selected - list_height / 2).min(search.len().saturating_sub(list_height))
    } else {
        0
    };
    let end = (start + list_height).min(search.len());
    let lines = (start..end)
        .filter_map(|index| {
            let path = search.relative_at(index)?;
            let highlighted = index == selected;
            let style = if highlighted {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if highlighted { "❯ " } else { "  " };
            Some(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(path.to_string(), style),
            ]))
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Text::from(lines)), chunks[1]);
}

pub fn draw_help(frame: &mut Frame) {
    let area = centered_rect(82, 72, frame.area());
    let lines = vec![
        Line::from(vec![Span::styled(
            "pad-sider keys",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::default(),
        Line::from("Global"),
        Line::from("  F10           toggle sider"),
        Line::from("  ?             show / hide this help"),
        Line::from("  q             quit sider UI"),
        Line::from("  r             refresh project state"),
        Line::from("  Tab           switch nav / Changes"),
        Line::from("  [ / ]         sider width: 35% / 50% / 65%"),
        Line::from("  + / - / 0     grow / shrink / reset left section"),
        Line::from("  t / d         focus Tree / Changes"),
        Line::from("  II            switch Tree / Index Map"),
        Line::from("  g / G         top / bottom"),
        Line::default(),
        Line::from("Tree"),
        Line::from("  j/k ↑/↓       move"),
        Line::from("  Enter/Space   expand or collapse directory"),
        Line::from("  Space         preview selected .md file"),
        Line::from("  i             open nearest index.md guide"),
        Line::from("  /             fuzzy search files"),
        Line::default(),
        Line::from("Index Map"),
        Line::from("  II            focus all project index.md files"),
        Line::from("  Enter/Space   preview selected index.md"),
        Line::from("  o             reveal selected index.md in tree"),
        Line::default(),
        Line::from("Preview"),
        Line::from("  q/Esc         close preview"),
        Line::from("  j/k ↑/↓       scroll"),
        Line::from("  g/G           top / bottom"),
        Line::from("  PgUp/PgDn     scroll right preview"),
        Line::default(),
        Line::from("Search"),
        Line::from("  type          filter files"),
        Line::from("  ↑/↓           move"),
        Line::from("  Enter         reveal file in tree"),
        Line::from("  Esc           cancel"),
    ];

    frame.render_widget(Clear, area);
    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(" help ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn focus_block(title: &str, focused: bool) -> Block<'static> {
    let mut block = Block::default()
        .title(title.to_string())
        .borders(Borders::ALL);
    if focused {
        block = block.border_style(Style::default().add_modifier(Modifier::BOLD));
    }
    block
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
