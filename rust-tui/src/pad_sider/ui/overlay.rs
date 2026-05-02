use super::super::{
    app::{App, MarkdownPreview},
    search::FileSearch,
};
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
    let paragraph = Paragraph::new(render_markdown(&preview.content))
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
