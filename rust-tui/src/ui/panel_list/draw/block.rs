use super::super::labels::{display_scope_title_label, special_view_title_label};
use crate::app::state::ThreadListView;
use crate::i18n::Locale;
use crate::theme::Theme;
use ratatui::{
    layout::Alignment,
    style::Style,
    widgets::{Block, BorderType, Borders},
};

pub(super) fn panel_list_block(
    locale: Locale,
    thread_list_view: ThreadListView,
    showing_live: bool,
    panel_is_focused: bool,
    item_count: usize,
    theme: &Theme,
) -> Block<'static> {
    let border_color = if panel_is_focused {
        theme.border_focused
    } else {
        theme.border
    };
    let focus_mark = if panel_is_focused { "●" } else { "○" };
    let title = if thread_list_view != ThreadListView::Normal {
        format!(
            " {} {} {} ",
            focus_mark,
            special_view_title_label(locale, thread_list_view),
            item_count
        )
    } else {
        format!(
            " {} {} {} ",
            focus_mark,
            display_scope_title_label(locale, showing_live),
            item_count
        )
    };

    Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(border_color))
}
