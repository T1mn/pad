use crate::theme::Theme;
use crate::ui::selection::SelectionItem;
use ratatui::style::{Color, Modifier, Style};

pub(super) fn marker_style(
    item: &SelectionItem,
    theme: &Theme,
    row_bg: Color,
    is_selected: bool,
) -> Style {
    if item.disabled {
        Style::default()
            .fg(theme.comment)
            .bg(row_bg)
            .add_modifier(Modifier::DIM)
    } else if is_selected {
        Style::default()
            .fg(theme.border_focused)
            .bg(row_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(row_bg)
    }
}

pub(super) fn value_style(
    item: &SelectionItem,
    theme: &Theme,
    row_bg: Color,
    is_selected: bool,
) -> Style {
    if item.disabled {
        Style::default()
            .fg(theme.comment)
            .bg(row_bg)
            .add_modifier(Modifier::DIM)
    } else if is_selected {
        Style::default()
            .fg(theme.highlight_fg)
            .bg(row_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.accent).bg(row_bg)
    }
}
