use super::metrics::{display_width, truncate_to_width};
use super::style::{sidebar_folder_bg, sidebar_folder_fg};
use crate::sidebar::SidebarFolderSummary;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Cell, Row},
};

pub(crate) fn build_folder_row(
    folder: &SidebarFolderSummary,
    is_selected: bool,
    content_width: usize,
    theme: &crate::theme::Theme,
    is_expanded: bool,
    is_hovered: bool,
) -> Row<'static> {
    let is_minimal = content_width < 10;
    let card_bg = sidebar_folder_bg(is_selected, theme);
    let unread = folder.has_unread_stop;
    let icon = if is_expanded { "▾" } else { "▸" };
    let card_width = content_width.saturating_sub(2);

    let mut spans = Vec::new();
    spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    spans.push(Span::styled(" ", Style::default().bg(card_bg)));

    let icon_style = Style::default()
        .fg(if is_selected {
            theme.accent
        } else if is_hovered {
            theme.border_focused
        } else {
            theme.accent
        })
        .bg(card_bg);
    spans.push(Span::styled(format!("{} ", icon), icon_style));

    if !is_minimal {
        let count = folder.thread_count.to_string();
        let count_width = display_width(&count);
        let label_width = card_width.saturating_sub(5 + count_width).clamp(1, 30);
        let label = truncate_to_width(&folder.label, label_width);
        let used_width = 1 + 2 + display_width(&label) + count_width;

        spans.push(Span::styled(
            label,
            folder_label_style(is_selected, unread, theme, card_bg),
        ));

        let fill_width = card_width.saturating_sub(used_width + 1);
        if fill_width > 0 {
            spans.push(Span::styled(
                " ".repeat(fill_width),
                Style::default().bg(card_bg),
            ));
        }

        spans.push(Span::styled(
            count,
            count_style(is_selected, unread, theme, card_bg),
        ));
    }

    spans.push(Span::styled(" ", Style::default().bg(card_bg)));
    spans.push(Span::styled(" ", Style::default().bg(theme.bg)));

    Row::new(vec![Cell::from(Text::from(vec![Line::from(spans)]))])
        .height(1)
        .style(Style::default().bg(theme.bg))
}

fn folder_label_style(
    is_selected: bool,
    _unread: bool,
    theme: &crate::theme::Theme,
    card_bg: ratatui::style::Color,
) -> Style {
    Style::default()
        .fg(sidebar_folder_fg(is_selected, theme))
        .bg(card_bg)
        .add_modifier(Modifier::BOLD)
}

fn count_style(
    is_selected: bool,
    unread: bool,
    theme: &crate::theme::Theme,
    card_bg: ratatui::style::Color,
) -> Style {
    let mut style = Style::default()
        .fg(if is_selected {
            theme.highlight_fg
        } else {
            theme.accent
        })
        .bg(card_bg);
    if unread {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

#[cfg(test)]
#[path = "folder_row_tests.rs"]
mod tests;
