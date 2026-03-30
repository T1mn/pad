use super::animation::breathing_badge_style;
use super::animation::breathing_badge_text;
use super::animation::thread_badge_breathes;
use super::metrics::{display_width, truncate_to_width};
use super::style::{
    badge_color, blend_color, maybe_bold, sidebar_card_bg, sidebar_subtitle_fg, sidebar_thread_fg,
};
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
) -> Row<'static> {
    let is_minimal = content_width < 12;
    let card_bg = sidebar_card_bg(is_selected, theme);
    let card_fg = sidebar_thread_fg(is_selected, theme);
    let badge_color = badge_color(thread.agent_type.clone(), theme);
    let is_working = thread_badge_breathes(&thread.state);
    let unread = thread.has_unread_stop;

    let mut lines = Vec::new();
    let card_width = content_width.saturating_sub(2);
    let inner_card_width = card_width.saturating_sub(2);
    let badge = if is_working {
        breathing_badge_text()
    } else {
        "• "
    };
    let badge_width = display_width(badge);

    let mut l1_spans = Vec::new();
    l1_spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    l1_spans.push(Span::styled(" ", Style::default().bg(card_bg)));
    l1_spans.push(Span::styled(
        badge,
        if is_working {
            breathing_badge_style(badge_color, card_bg, card_bg)
        } else {
            Style::default().fg(badge_color).bg(card_bg)
        },
    ));

    if !is_minimal {
        let meta = if thread.pinned {
            " pin"
        } else if thread.is_live() {
            " live"
        } else {
            ""
        };
        let meta_width = display_width(meta);
        let title_width = inner_card_width
            .saturating_sub(badge_width + meta_width + 1)
            .clamp(1, 52);
        let compact_title = truncate_to_width(&thread.title, title_width);
        let used_width = badge_width + display_width(&compact_title) + meta_width;
        let title_color = if thread.state == AgentState::Waiting && !is_selected {
            theme.success
        } else {
            card_fg
        };

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

        let fill_width = inner_card_width.saturating_sub(used_width + 1);
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
                        blend_color(theme.highlight_fg, theme.comment, 0.42)
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
    l2_spans.push(Span::styled(" ", Style::default().bg(card_bg)));
    l2_spans.extend(subtitle_spans);
    l2_spans.push(Span::styled(" ", Style::default().bg(card_bg)));
    l2_spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
    lines.push(Line::from(l2_spans));

    Row::new(vec![Cell::from(Text::from(lines))])
        .height(2)
        .style(Style::default().bg(theme.bg))
}

pub(crate) fn thread_subtitle(thread: &SidebarThread) -> String {
    thread
        .subtitle
        .as_deref()
        .or(thread.last_user_prompt.as_deref())
        .or_else(|| {
            thread
                .cached_preview_turns
                .first()
                .map(|turn| turn.question.as_str())
        })
        .or(thread.last_assistant_message.as_deref())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn build_thread_subtitle_spans(
    thread: &SidebarThread,
    subtitle: &str,
    color: ratatui::style::Color,
    row_bg: ratatui::style::Color,
    content_width: usize,
) -> Vec<Span<'static>> {
    let prefix = " ";
    let prefix_width = display_width(prefix);
    let tags_text = thread_tags_text(thread, content_width / 3);
    let tags_width = display_width(&tags_text);
    let available_width = content_width.saturating_sub(prefix_width);
    let subtitle_max_width = available_width
        .saturating_sub(tags_width + if tags_width > 0 { 1 } else { 0 })
        .max(1);
    let compact_subtitle = truncate_to_width(subtitle, subtitle_max_width);
    let subtitle_width = display_width(&compact_subtitle);
    let spacer_width = content_width
        .saturating_sub(prefix_width + subtitle_width + tags_width)
        .max(1);
    let mut spans = vec![
        Span::styled(prefix.to_string(), Style::default().bg(row_bg)),
        Span::styled(
            compact_subtitle,
            Style::default()
                .fg(color)
                .bg(row_bg)
                .add_modifier(Modifier::DIM),
        ),
    ];
    spans.push(Span::styled(
        " ".repeat(spacer_width),
        Style::default().bg(row_bg),
    ));
    if !tags_text.is_empty() {
        spans.push(Span::styled(
            tags_text,
            Style::default()
                .fg(color)
                .bg(row_bg)
                .add_modifier(Modifier::DIM),
        ));
    }
    spans
}

fn thread_tags_text(thread: &SidebarThread, max_width: usize) -> String {
    if thread.tags.is_empty() || max_width == 0 {
        return String::new();
    }

    let mut rendered = String::new();
    for tag in &thread.tags {
        let candidate = if rendered.is_empty() {
            format!("#{}", tag)
        } else {
            format!("{} #{}", rendered, tag)
        };
        if display_width(&candidate) > max_width {
            break;
        }
        rendered = candidate;
    }
    rendered
}
