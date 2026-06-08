use super::layout::ThreadRowLayout;
use crate::sidebar::SidebarThread;
use crate::ui::panel_list::style::{sidebar_card_bg, sidebar_subtitle_fg};
use crate::ui::panel_list::thread_subtitle::{build_thread_subtitle_spans, thread_subtitle};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(super) fn render_thread_subtitle_line(
    thread: &SidebarThread,
    is_selected: bool,
    layout: &ThreadRowLayout,
    theme: &crate::theme::Theme,
) -> Line<'static> {
    let card_bg = sidebar_card_bg(is_selected, theme);
    let subtitle_spans = if layout.is_minimal {
        vec![Span::styled(
            " ".repeat(layout.inner_card_width),
            Style::default().bg(card_bg),
        )]
    } else {
        let subtitle = thread_subtitle(thread);
        build_thread_subtitle_spans(
            thread,
            &subtitle,
            sidebar_subtitle_fg(is_selected, theme),
            card_bg,
            layout.inner_card_width,
        )
    };

    let mut spans = Vec::new();
    spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    spans.push(Span::styled("  ", Style::default().bg(card_bg)));
    spans.extend(subtitle_spans);
    spans.push(Span::styled(" ", Style::default().bg(card_bg)));
    spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    Line::from(spans)
}
