use crate::app::App;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw_agent_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let l = app.locale;
    let active = app.panels.iter().filter(|p| p.is_active).count();
    let total = app.panels.len();
    let tmpl = crate::i18n::t(l, "panel.agent_count");
    let text = format!(
        " {} ",
        tmpl.replacen("{}", &total.to_string(), 1)
            .replacen("{}", &active.to_string(), 1)
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
        .border_style(Style::default().fg(app.theme.border));
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}
