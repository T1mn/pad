use super::animation::breathing_badge_style;
use super::animation::breathing_badge_text;
use super::animation::thread_badge_breathes;
use super::metrics::{display_width, truncate_to_width};
use super::style::{
    badge_color, blend_color, maybe_bold, sidebar_card_bg, sidebar_subtitle_fg, sidebar_thread_fg,
};
use super::thread_subtitle::{build_thread_subtitle_spans, thread_subtitle};
use crate::model::AgentState;
use crate::sidebar::SidebarThread;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Cell, Row},
};

pub(crate) fn build_thread_row(
    thread: &SidebarThread,
    is_selected: bool,
    content_width: usize,
    theme: &crate::theme::Theme,
    jump_badge: Option<usize>,
) -> Row<'static> {
    const OUTER_PAD: usize = 1;
    const CARD_LEFT_PAD: usize = 2;
    const CARD_RIGHT_PAD: usize = 1;
    const JUMP_BADGE_SLOT_WIDTH: usize = 4;
    let is_minimal = content_width < 12;
    let card_bg = sidebar_card_bg(is_selected, theme);
    let card_fg = sidebar_thread_fg(is_selected, theme);
    let badge_color = badge_color(thread.agent_type.clone(), theme);
    let is_working = thread_badge_breathes(&thread.state);
    let unread = thread.has_unread_stop;

    let mut lines = Vec::new();
    let card_width = content_width.saturating_sub(OUTER_PAD * 2);
    let inner_card_width = card_width.saturating_sub(CARD_LEFT_PAD + CARD_RIGHT_PAD);
    let badge = if is_working {
        breathing_badge_text()
    } else {
        "• "
    };
    let badge_width = display_width(badge);

    let mut l1_spans = Vec::new();
    l1_spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    l1_spans.push(Span::styled("  ", Style::default().bg(card_bg)));
    l1_spans.push(Span::styled(
        badge,
        if is_working {
            breathing_badge_style(badge_color, card_bg, card_bg)
        } else {
            Style::default().fg(badge_color).bg(card_bg)
        },
    ));

    if !is_minimal {
        let jump_badge_text = format_jump_badge(jump_badge, JUMP_BADGE_SLOT_WIDTH);
        let jump_badge_width = display_width(&jump_badge_text);
        let meta = if thread.pinned {
            " pin"
        } else if thread.is_live() {
            " live"
        } else {
            ""
        };
        let meta_width = display_width(meta);
        let title_width = inner_card_width
            .saturating_sub(badge_width + jump_badge_width + meta_width)
            .clamp(1, 52);
        let compact_title = truncate_to_width(&thread.title, title_width);
        let used_width =
            badge_width + jump_badge_width + display_width(&compact_title) + meta_width;
        let title_color = if thread.state == AgentState::Waiting && !is_selected {
            theme.success
        } else {
            card_fg
        };

        l1_spans.push(Span::styled(
            jump_badge_text,
            Style::default()
                .fg(if is_selected {
                    blend_color(theme.highlight_fg, theme.comment, 0.72)
                } else {
                    theme.comment
                })
                .bg(card_bg)
                .add_modifier(Modifier::DIM),
        ));

        l1_spans.push(Span::styled(
            compact_title,
            maybe_bold(
                Style::default()
                    .fg(title_color)
                    .bg(card_bg)
                    .add_modifier(if is_selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                unread,
            ),
        ));

        let fill_width = inner_card_width.saturating_sub(used_width);
        if fill_width > 0 {
            l1_spans.push(Span::styled(
                " ".repeat(fill_width),
                Style::default().bg(card_bg),
            ));
        }

        if !meta.is_empty() {
            l1_spans.push(Span::styled(
                meta,
                Style::default()
                    .fg(if is_selected {
                        blend_color(theme.highlight_fg, theme.comment, 0.66)
                    } else {
                        theme.comment
                    })
                    .bg(card_bg)
                    .add_modifier(Modifier::DIM),
            ));
        }
    }
    l1_spans.push(Span::styled(" ", Style::default().bg(card_bg)));
    l1_spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    lines.push(Line::from(l1_spans));

    let subtitle = thread_subtitle(thread);
    let subtitle_spans = if is_minimal {
        vec![Span::styled(
            " ".repeat(inner_card_width),
            Style::default().bg(card_bg),
        )]
    } else {
        build_thread_subtitle_spans(
            thread,
            &subtitle,
            sidebar_subtitle_fg(is_selected, theme),
            card_bg,
            inner_card_width,
        )
    };

    let mut l2_spans = Vec::new();
    l2_spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    l2_spans.push(Span::styled("  ", Style::default().bg(card_bg)));
    l2_spans.extend(subtitle_spans);
    l2_spans.push(Span::styled(" ", Style::default().bg(card_bg)));
    l2_spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    lines.push(Line::from(l2_spans));

    Row::new(vec![Cell::from(Text::from(lines))])
        .height(2)
        .style(Style::default().bg(theme.bg))
}

pub(crate) fn format_jump_badge(jump_badge: Option<usize>, slot_width: usize) -> String {
    let content = jump_badge
        .filter(|number| (1..=9).contains(number))
        .map(|number| format!("#{}", number))
        .unwrap_or_default();
    format!("{content:<slot_width$}")
}
