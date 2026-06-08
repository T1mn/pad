mod labels;
mod row;
mod values;

use super::common::render_modal_surface;
use crate::app::App;
use labels::footer_text;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph, Table},
    Frame,
};
use row::telegram_row;
use values::telegram_rows;

pub fn draw_telegram_settings_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = crate::ui::layout::popup_area(72, 13, f.area());
    render_modal_surface(f, area, theme);
    draw_telegram_settings_content(f, app, area, true);
}

pub(super) fn draw_telegram_settings_content(
    f: &mut Frame,
    app: &App,
    area: Rect,
    show_footer: bool,
) {
    let theme = &app.theme;
    let inner = if show_footer {
        let block = Block::default()
            .title(" ✈ Telegram ".to_string())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(theme.accent));
        f.render_widget(block, area);
        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        }
    } else {
        area
    };

    let rows = telegram_rows(app)
        .into_iter()
        .map(|row| telegram_row(app, row))
        .collect::<Vec<_>>();
    let table = Table::new(rows, [Constraint::Length(18), Constraint::Min(0)]);
    f.render_widget(table, inner);

    if show_footer {
        let footer = Paragraph::new(footer_text(app.locale, app.telegram_editing))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.comment));
        let footer_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(2),
            width: area.width,
            height: 1,
        };
        f.render_widget(footer, footer_area);
    }
}
