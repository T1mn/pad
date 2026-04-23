use super::super::common::render_modal_surface;
use crate::app::App;
use crate::tree::AgentLauncher;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Cell, Row, Table},
    Frame,
};

pub(super) fn draw_agent_launcher(f: &mut Frame, app: &App, launcher: &AgentLauncher, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let popup_width = 50;
    let popup_height = 12;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(
        area.x + popup_x,
        area.y + popup_y,
        popup_width,
        popup_height,
    );

    render_modal_surface(f, popup_area, theme);

    let items: Vec<Row> = launcher
        .agents
        .iter()
        .enumerate()
        .map(|(idx, (name, _))| {
            let prefix = if idx == launcher.selected {
                "❯ "
            } else {
                "  "
            };
            let cells = vec![Cell::from(format!("{}{}", prefix, name))];
            let style = if idx == launcher.selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            Row::new(cells).style(style)
        })
        .collect();

    let title = format!(
        " {} {} ",
        crate::i18n::t(locale, "agent_launcher.title"),
        launcher.target_dir.display()
    );
    let block = Block::default()
        .title(title)
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));

    let table = Table::new(items, [Constraint::Percentage(100)]).block(block);

    f.render_widget(table, popup_area);
}
