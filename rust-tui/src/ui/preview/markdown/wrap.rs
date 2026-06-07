use super::super::common::display_width;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub(crate) fn total_span_count(lines: &[Line<'_>]) -> usize {
    lines.iter().map(|line| line.spans.len()).sum()
}

pub(crate) fn flatten_lines_for_smooth_scrolling(
    lines: Vec<Line<'static>>,
    default_fg: Color,
    surface_bg: Color,
) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(|line| flatten_line_for_smooth_scrolling(line, default_fg, surface_bg))
        .collect()
}

fn flatten_line_for_smooth_scrolling(
    line: Line<'static>,
    default_fg: Color,
    surface_bg: Color,
) -> Line<'static> {
    let style = dominant_detail_line_style(&line, default_fg, surface_bg);
    let text = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    Line::from(Span::styled(text, style))
}

fn dominant_detail_line_style(line: &Line<'_>, default_fg: Color, surface_bg: Color) -> Style {
    let mut fg = None;
    let mut bg = None;
    let mut modifiers = Modifier::empty();

    for span in &line.spans {
        if fg.is_none() {
            fg = span.style.fg;
        }
        if bg.is_none() {
            bg = span.style.bg;
        }
        modifiers |= span.style.add_modifier;
    }

    let mut style = Style::default()
        .fg(fg.unwrap_or(default_fg))
        .bg(bg.unwrap_or(surface_bg));
    if !modifiers.is_empty() {
        style = style.add_modifier(modifiers);
    }
    style
}

pub(crate) fn wrap_text_to_width(
    text: &Text<'_>,
    width: usize,
    default_fg: Color,
    surface_bg: Color,
) -> Vec<Line<'static>> {
    let mut wrapped = Vec::new();
    for line in &text.lines {
        let styled = apply_surface_style(line, default_fg, surface_bg);
        wrapped.extend(wrap_styled_line(&styled, width));
    }

    if wrapped.is_empty() {
        wrapped.push(Line::default());
    }

    wrapped
}

fn apply_surface_style(line: &Line<'_>, default_fg: Color, surface_bg: Color) -> Line<'static> {
    let spans = line
        .spans
        .iter()
        .map(|span| {
            let mut style = span.style;
            if style.fg.is_none() {
                style = style.fg(default_fg);
            }
            if style.bg.is_none() {
                style = style.bg(surface_bg);
            }
            Span::styled(span.content.to_string(), style)
        })
        .collect::<Vec<_>>();
    Line::from(spans)
}

pub(crate) fn wrap_styled_line(line: &Line<'_>, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![Line::default()];
    }
    if line.spans.is_empty() {
        return vec![Line::default()];
    }

    let mut lines = Vec::new();
    let mut current_spans = Vec::new();
    let mut current_width = 0usize;

    for span in &line.spans {
        let style = span.style;
        let mut buffer = String::new();

        for ch in span.content.chars() {
            let rendered = if ch == '\t' {
                "    ".to_string()
            } else {
                ch.to_string()
            };
            let ch_width = display_width(&rendered).max(1);

            if current_width > 0 && current_width + ch_width > width {
                if !buffer.is_empty() {
                    current_spans.push(Span::styled(std::mem::take(&mut buffer), style));
                }
                lines.push(Line::from(std::mem::take(&mut current_spans)));
                current_width = 0;
            }

            buffer.push_str(&rendered);
            current_width += ch_width;
        }

        if !buffer.is_empty() {
            current_spans.push(Span::styled(buffer, style));
        }
    }

    if current_spans.is_empty() {
        lines.push(Line::default());
    } else {
        lines.push(Line::from(current_spans));
    }

    lines
}
