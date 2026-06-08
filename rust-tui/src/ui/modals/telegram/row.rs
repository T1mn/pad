use crate::app::App;
use ratatui::{
    style::{Modifier, Style},
    widgets::{Cell, Row},
};

use super::values::TelegramRowValue;

pub(super) fn telegram_row(app: &App, row: TelegramRowValue) -> Row<'static> {
    let theme = &app.theme;
    let is_selected = row.editable && row.field_idx == app.telegram_selected_field;
    let name_style = if is_selected {
        Style::default()
            .bg(theme.highlight_bg)
            .fg(theme.highlight_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg)
    };
    let value_style = if is_selected {
        Style::default().bg(theme.highlight_bg).fg(theme.accent)
    } else if row.editable {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.comment)
    };
    Row::new(vec![
        Cell::from(row.name).style(name_style),
        Cell::from(row.value).style(value_style),
    ])
}
