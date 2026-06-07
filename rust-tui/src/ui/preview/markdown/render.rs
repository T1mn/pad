use super::super::common::{blend_color, display_width, fallback_color};
use crate::theme::Theme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub(crate) fn detail_surface(theme: &Theme) -> Color {
    let base = fallback_color(theme.bg, theme.highlight_bg);
    let highlight = fallback_color(theme.highlight_bg, theme.border);
    blend_color(highlight, base, 0.24)
}

pub(crate) fn render_detail_separator_line(
    width: usize,
    label: &str,
    label_fg: Color,
    label_bg: Color,
    line_color: Color,
    surface_bg: Color,
) -> Line<'static> {
    let badge = preview_badge(label, label_fg, label_bg);
    let badge_width = display_width(badge.content.as_ref());
    let inner = width.saturating_sub(4);
    let gap = 2usize;
    let used = badge_width + gap;
    let left = inner.saturating_sub(used) / 2;
    let right = inner.saturating_sub(used + left);
    let line_style = Style::default().fg(line_color).bg(surface_bg);
    let fill = Style::default().bg(surface_bg);

    Line::from(vec![
        Span::styled("  ", fill),
        Span::styled("─".repeat(left), line_style),
        Span::styled(" ".repeat(gap / 2), fill),
        badge,
        Span::styled(" ".repeat(gap - gap / 2), fill),
        Span::styled("─".repeat(right), line_style),
        Span::styled("  ", fill),
    ])
}

fn preview_badge(label: &str, fg: Color, bg: Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

pub(crate) fn render_detail_padding_line(width: usize, surface_bg: Color) -> Line<'static> {
    Line::from(Span::styled(
        " ".repeat(width),
        Style::default().bg(surface_bg),
    ))
}

pub(crate) fn render_detail_content_line(
    line: Line<'static>,
    content_width: usize,
    surface_bg: Color,
) -> Line<'static> {
    let used_width = line
        .spans
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum::<usize>()
        .min(content_width);
    let pad = content_width.saturating_sub(used_width);
    let fill = Style::default().bg(surface_bg);
    let mut spans = vec![Span::styled("  ", fill)];
    spans.extend(line.spans);
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), fill));
    }
    spans.push(Span::styled("  ", fill));
    Line::from(spans)
}
