use super::metrics::{display_width, truncate_to_width};
use super::style::blend_color;
use crate::sidebar::SidebarThread;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

pub(crate) fn thread_subtitle(thread: &SidebarThread) -> String {
    thread
        .last_user_prompt
        .as_deref()
        .or(thread.subtitle.as_deref())
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

pub(super) fn build_thread_subtitle_spans(
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
    let tags_gap_width = if tags_width > 0 { 1 } else { 0 };
    let available_width = content_width.saturating_sub(prefix_width);
    let subtitle_max_width = available_width
        .saturating_sub(tags_width + tags_gap_width)
        .max(1);
    let compact_subtitle = truncate_to_width(subtitle, subtitle_max_width);
    let subtitle_width = display_width(&compact_subtitle);
    let spacer_width =
        content_width.saturating_sub(prefix_width + subtitle_width + tags_width + tags_gap_width);
    let mut spans = vec![
        Span::styled(prefix.to_string(), Style::default().bg(row_bg)),
        Span::styled(compact_subtitle, Style::default().fg(color).bg(row_bg)),
    ];
    spans.push(Span::styled(
        " ".repeat(spacer_width),
        Style::default().bg(row_bg),
    ));
    if !tags_text.is_empty() {
        spans.push(Span::styled(" ", Style::default().bg(row_bg)));
        spans.push(Span::styled(
            tags_text,
            Style::default()
                .fg(blend_color(color, row_bg, 0.84))
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
