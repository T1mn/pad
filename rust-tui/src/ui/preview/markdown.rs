use super::common::{blend_color, fallback_color};
use crate::theme::Theme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use tui_markdown::{Options as MarkdownOptions, StyleSheet};

#[derive(Clone)]
pub(crate) struct PreviewMarkdownStyleSheet {
    theme: Theme,
}

impl PreviewMarkdownStyleSheet {
    pub(crate) fn new(theme: &Theme) -> Self {
        Self {
            theme: theme.clone(),
        }
    }
}

impl StyleSheet for PreviewMarkdownStyleSheet {
    fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::default()
                .fg(self.theme.keyword)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            2 => Style::default()
                .fg(self.theme.accent)
                .add_modifier(Modifier::BOLD),
            3 => Style::default()
                .fg(self.theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            _ => Style::default()
                .fg(self.theme.comment)
                .add_modifier(Modifier::ITALIC),
        }
    }

    fn code(&self) -> Style {
        inline_code_style(&self.theme)
    }

    fn link(&self) -> Style {
        Style::default()
            .fg(self.theme.accent)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn blockquote(&self) -> Style {
        Style::default().fg(self.theme.comment)
    }

    fn heading_meta(&self) -> Style {
        Style::default()
            .fg(self.theme.comment)
            .add_modifier(Modifier::DIM)
    }

    fn metadata_block(&self) -> Style {
        Style::default().fg(self.theme.comment)
    }
}

pub(crate) fn markdown_options(theme: &Theme) -> MarkdownOptions<PreviewMarkdownStyleSheet> {
    MarkdownOptions::new(PreviewMarkdownStyleSheet::new(theme))
}

pub(crate) fn detail_surface(theme: &Theme) -> Color {
    let base = fallback_color(theme.bg, theme.highlight_bg);
    let highlight = fallback_color(theme.highlight_bg, theme.border);
    blend_color(highlight, base, 0.52)
}

pub(crate) fn normalize_session_detail_markdown(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 1 {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len() + lines.len());
    let mut in_fenced_code = false;

    for (idx, line) in lines.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(line);

        let trimmed = line.trim();
        if is_fence_marker(trimmed) {
            in_fenced_code = !in_fenced_code;
            continue;
        }
        if in_fenced_code {
            continue;
        }

        let Some(next) = lines.get(idx + 1) else {
            continue;
        };
        if should_insert_session_paragraph_gap(line, next) {
            out.push('\n');
        }
    }

    out
}

pub(crate) fn should_insert_session_paragraph_gap(current: &str, next: &str) -> bool {
    let current = current.trim();
    let next = next.trim();
    if current.is_empty() || next.is_empty() {
        return false;
    }
    if is_fence_marker(current) || is_fence_marker(next) {
        return false;
    }
    if is_setext_underline(current) || is_setext_underline(next) {
        return false;
    }
    if is_markdown_structural_line(current) || is_markdown_structural_line(next) {
        return false;
    }
    true
}

pub(crate) fn is_fence_marker(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

pub(crate) fn is_setext_underline(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && trimmed.chars().all(|ch| matches!(ch, '-' | '='))
}

pub(crate) fn is_markdown_structural_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#')
        || trimmed.starts_with('>')
        || trimmed.starts_with('|')
        || trimmed.starts_with("    ")
        || trimmed.starts_with('\t')
        || trimmed.starts_with("- [")
        || trimmed.starts_with("* [")
        || trimmed.starts_with("+ [")
        || trimmed.starts_with("---")
        || trimmed.starts_with("***")
    {
        return true;
    }

    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return true;
    }

    let mut chars = trimmed.chars().peekable();
    let mut saw_digit = false;
    while let Some(ch) = chars.peek() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            chars.next();
        } else {
            break;
        }
    }
    if saw_digit && matches!(chars.next(), Some('.' | ')')) && matches!(chars.next(), Some(' ')) {
        return true;
    }

    false
}

pub(crate) fn inline_code_style(theme: &Theme) -> Style {
    Style::default()
        .fg(derived_inline_code_fg(theme))
        .bg(derived_inline_code_bg(theme))
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
    let badge_width = super::common::display_width(badge.content.as_ref());
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

pub(crate) fn preview_badge(label: &str, fg: Color, bg: Color) -> Span<'static> {
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
        .map(|span| super::common::display_width(span.content.as_ref()))
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

pub(crate) fn apply_surface_style(
    line: &Line<'_>,
    default_fg: Color,
    surface_bg: Color,
) -> Line<'static> {
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

pub(crate) fn flatten_line_for_smooth_scrolling(
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

pub(crate) fn dominant_detail_line_style(
    line: &Line<'_>,
    default_fg: Color,
    surface_bg: Color,
) -> Style {
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
            let ch_width = super::common::display_width(&rendered).max(1);

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

pub(crate) fn derived_inline_code_bg(theme: &Theme) -> Color {
    let base = fallback_color(theme.bg, theme.highlight_bg);
    let surface = fallback_color(theme.highlight_bg, theme.border);
    blend_color(surface, base, 0.72)
}

pub(crate) fn derived_inline_code_fg(theme: &Theme) -> Color {
    let base = fallback_color(theme.fg, theme.highlight_fg);
    let accent = fallback_color(theme.accent, base);
    blend_color(accent, base, 0.28)
}

pub(crate) fn tokenize_inline_code(
    text: &str,
    base_style: Style,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find('`') {
        let before = &rest[..start];
        if !before.is_empty() {
            spans.push(Span::styled(before.to_string(), base_style));
        }

        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            spans.push(Span::styled(rest.to_string(), base_style));
            return spans;
        };

        let code = &after_start[..end];
        if !code.is_empty() {
            spans.push(Span::styled(code.to_string(), inline_code_style(theme)));
        }
        rest = &after_start[end + 1..];
    }

    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_string(), base_style));
    }

    spans
}

pub(crate) fn retokenize_inline_code(
    spans: Vec<Span<'static>>,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    for span in spans {
        let content = span.content.into_owned();
        if content.contains('`') {
            out.extend(tokenize_inline_code(&content, span.style, theme));
        } else {
            out.push(Span::styled(content, span.style));
        }
    }
    out
}

pub(crate) fn format_line(line: &str, theme: &Theme) -> Vec<Span<'static>> {
    let stripped = line.trim();

    let user_markers = ["$", "#", "❯", ">", "%"];
    for marker in &user_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            let mut spans = vec![Span::styled(
                (*marker).to_string(),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(tokenize_inline_code(
                &format!(" {}", content),
                Style::default().fg(theme.success),
                theme,
            ));
            return spans;
        }
    }

    let ai_markers = ["●", "•", "💫", "🤖", "🟣", "🔵", "🟢", "⚡"];
    for marker in &ai_markers {
        if stripped.starts_with(marker) {
            let content = stripped.strip_prefix(marker).unwrap_or("").trim();
            let mut spans = vec![Span::styled(
                (*marker).to_string(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(tokenize_inline_code(
                &format!(" {}", content),
                Style::default().fg(theme.accent),
                theme,
            ));
            return spans;
        }
    }

    if stripped.to_lowercase().contains("error") || stripped.to_lowercase().contains("failed") {
        return tokenize_inline_code(line, Style::default().fg(theme.error), theme);
    }

    if stripped.to_lowercase().contains("success")
        || stripped.to_lowercase().contains("done")
        || stripped.contains("✓")
    {
        return tokenize_inline_code(line, Style::default().fg(theme.success), theme);
    }

    tokenize_inline_code(line, Style::default(), theme)
}
