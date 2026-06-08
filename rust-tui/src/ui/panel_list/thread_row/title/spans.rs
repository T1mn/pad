use crate::model::AgentState;
use crate::sidebar::SidebarThread;
use crate::ui::panel_list::style::{blend_color, maybe_bold, sidebar_thread_fg};
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

pub(super) fn render_jump_badge(
    jump_badge_text: &str,
    is_selected: bool,
    theme: &crate::theme::Theme,
    card_bg: ratatui::style::Color,
) -> Span<'static> {
    Span::styled(
        jump_badge_text.to_string(),
        Style::default()
            .fg(if is_selected {
                blend_color(theme.highlight_fg, theme.comment, 0.72)
            } else {
                theme.comment
            })
            .bg(card_bg)
            .add_modifier(Modifier::DIM),
    )
}

pub(super) fn render_title_text(
    thread: &SidebarThread,
    compact_title: &str,
    is_selected: bool,
    theme: &crate::theme::Theme,
    card_bg: ratatui::style::Color,
) -> Span<'static> {
    let title_color = if thread.state == AgentState::Waiting && !is_selected {
        theme.success
    } else {
        sidebar_thread_fg(is_selected, theme)
    };
    Span::styled(
        compact_title.to_string(),
        maybe_bold(
            Style::default()
                .fg(title_color)
                .bg(card_bg)
                .add_modifier(if is_selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            thread.has_unread_stop,
        ),
    )
}

pub(super) fn render_meta(
    meta: &str,
    is_selected: bool,
    theme: &crate::theme::Theme,
    card_bg: ratatui::style::Color,
) -> Span<'static> {
    Span::styled(
        meta.to_string(),
        Style::default()
            .fg(if is_selected {
                blend_color(theme.highlight_fg, theme.comment, 0.66)
            } else {
                theme.comment
            })
            .bg(card_bg)
            .add_modifier(Modifier::DIM),
    )
}
