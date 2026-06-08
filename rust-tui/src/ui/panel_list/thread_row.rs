use crate::sidebar::SidebarThread;
use ratatui::{
    style::Style,
    text::Text,
    widgets::{Cell, Row},
};

mod layout;
mod subtitle;
mod title;

#[cfg(test)]
pub(crate) use title::format_jump_badge;

pub(crate) fn build_thread_row(
    thread: &SidebarThread,
    is_selected: bool,
    content_width: usize,
    theme: &crate::theme::Theme,
    jump_badge: Option<usize>,
) -> Row<'static> {
    let layout = layout::thread_row_layout(content_width);
    let lines = vec![
        title::render_thread_title_line(thread, is_selected, &layout, theme, jump_badge),
        subtitle::render_thread_subtitle_line(thread, is_selected, &layout, theme),
    ];

    Row::new(vec![Cell::from(Text::from(lines))])
        .height(2)
        .style(Style::default().bg(theme.bg))
}
