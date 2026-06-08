use super::super::super::telegram::draw_telegram_settings_content;
use crate::app::App;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(in crate::ui::modals::settings) fn draw_telegram_detail(f: &mut Frame, app: &App, area: Rect) {
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Telegram",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        header_area,
    );
    draw_telegram_settings_content(f, app, body_area, false);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "j/k move · Enter/Space edit · r restart · Esc back",
            Style::default()
                .fg(app.theme.comment)
                .add_modifier(Modifier::DIM),
        ))),
        footer_area,
    );
}
