mod body;
mod text;

use crate::app::App;
use body::{mode_span, status_body};
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use text::{display_width, format_status_remainder};

pub fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let background = Style::default().bg(theme.highlight_bg).fg(theme.status_fg);
    let mode_span = mode_span(app);
    let mode_width = display_width(mode_span.content.as_ref());
    let body_width = area.width.saturating_sub(mode_width as u16);
    let body = status_body(app, body_width);

    let line = Line::from(vec![
        mode_span,
        Span::styled(
            format_status_remainder(&body, area.width, mode_width),
            background,
        ),
    ]);
    let status_bar = Paragraph::new(line)
        .style(background)
        .alignment(Alignment::Left);
    f.render_widget(status_bar, area);
}

#[cfg(test)]
#[path = "status_bar_tests.rs"]
mod tests;
