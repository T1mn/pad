use super::layout::{ThreadRowLayout, JUMP_BADGE_SLOT_WIDTH};
use crate::sidebar::SidebarThread;
use crate::ui::panel_list::metrics::{display_width, truncate_to_width};
use crate::ui::panel_list::style::sidebar_card_bg;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

mod badge;
mod spans;

use badge::render_badge;
use spans::{render_jump_badge, render_meta, render_title_text};

pub(super) fn render_thread_title_line(
    thread: &SidebarThread,
    is_selected: bool,
    layout: &ThreadRowLayout,
    theme: &crate::theme::Theme,
    jump_badge: Option<usize>,
) -> Line<'static> {
    let card_bg = sidebar_card_bg(is_selected, theme);
    let mut spans = title_shell_prefix(theme.bg, card_bg);
    let badge = render_badge(thread, theme, card_bg);
    let badge_width = display_width(badge.text);
    spans.push(Span::styled(badge.text, badge.style));

    if !layout.is_minimal {
        spans.extend(render_title_body(
            thread,
            is_selected,
            layout.inner_card_width,
            badge_width,
            theme,
            card_bg,
            jump_badge,
        ));
    }

    spans.push(Span::styled(" ", Style::default().bg(card_bg)));
    spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    Line::from(spans)
}

fn title_shell_prefix(
    theme_bg: ratatui::style::Color,
    card_bg: ratatui::style::Color,
) -> Vec<Span<'static>> {
    vec![
        Span::styled(" ", Style::default().bg(theme_bg)),
        Span::styled("  ", Style::default().bg(card_bg)),
    ]
}

fn render_title_body(
    thread: &SidebarThread,
    is_selected: bool,
    inner_card_width: usize,
    badge_width: usize,
    theme: &crate::theme::Theme,
    card_bg: ratatui::style::Color,
    jump_badge: Option<usize>,
) -> Vec<Span<'static>> {
    let jump_badge_text = format_jump_badge(jump_badge, JUMP_BADGE_SLOT_WIDTH);
    let jump_badge_width = display_width(&jump_badge_text);
    let meta = title_meta(thread);
    let meta_width = display_width(meta);
    let title_width = inner_card_width
        .saturating_sub(badge_width + jump_badge_width + meta_width)
        .clamp(1, 52);
    let compact_title = truncate_to_width(&thread.title, title_width);
    let used_width = badge_width + jump_badge_width + display_width(&compact_title) + meta_width;

    let mut spans = vec![render_jump_badge(
        &jump_badge_text,
        is_selected,
        theme,
        card_bg,
    )];
    spans.push(render_title_text(
        thread,
        &compact_title,
        is_selected,
        theme,
        card_bg,
    ));

    let fill_width = inner_card_width.saturating_sub(used_width);
    if fill_width > 0 {
        spans.push(Span::styled(
            " ".repeat(fill_width),
            Style::default().bg(card_bg),
        ));
    }

    if !meta.is_empty() {
        spans.push(render_meta(meta, is_selected, theme, card_bg));
    }
    spans
}

fn title_meta(thread: &SidebarThread) -> &'static str {
    if thread.pinned {
        " pin"
    } else if thread.is_live() {
        " live"
    } else {
        ""
    }
}

pub(crate) fn format_jump_badge(jump_badge: Option<usize>, slot_width: usize) -> String {
    let content = jump_badge
        .filter(|number| (1..=9).contains(number))
        .map(|number| format!("#{}", number))
        .unwrap_or_default();
    format!("{content:<slot_width$}")
}
