use super::FuzzyPicker;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub(super) fn draw_picker(picker: &FuzzyPicker, f: &mut ratatui::Frame) {
    let area = centered_rect(70, 70, f.area());

    // Clear background
    f.render_widget(Clear, area);

    // Main block
    let block = Block::default()
        .title(" Select Directory ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = area.inner(Margin::new(2, 1));

    // Split into query area and list area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    // Query input
    let query_block = Block::default()
        .title(" Filter · Shift+Delete clear ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let query_text = Paragraph::new(picker.query.clone())
        .block(query_block)
        .wrap(Wrap { trim: false });

    f.render_widget(query_text, chunks[0]);
    render_items(picker, f, chunks[1]);

    // Render border last
    f.render_widget(block, area);

    // Help text at bottom
    let help = Paragraph::new("Up/Down: navigate | type: filter | Enter: select | Esc: cancel")
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

fn render_items(picker: &FuzzyPicker, f: &mut ratatui::Frame, area: Rect) {
    let list_height = area.height as usize;
    let start_idx = if picker.selected > list_height / 2 {
        (picker.selected - list_height / 2).min(picker.filtered.len().saturating_sub(list_height))
    } else {
        0
    };
    let end_idx = (start_idx + list_height).min(picker.filtered.len());

    let visible_items: Vec<Line> = picker.filtered[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(idx, (item, _score))| {
            let actual_idx = start_idx + idx;
            let style = if actual_idx == picker.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if actual_idx == picker.selected {
                "❯ "
            } else {
                "  "
            };
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
        f.render_widget(empty_text, area);
    } else {
        let text = ratatui::text::Text::from(visible_items);
        let list_text = Paragraph::new(text);
        f.render_widget(list_text, area);
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
