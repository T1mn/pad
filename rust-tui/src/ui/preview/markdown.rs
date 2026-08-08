mod inline {
    use super::style::inline_code_style;
    use crate::text_match::contains_ignore_case;
    use crate::theme::Theme;
    use ratatui::{
        style::{Modifier, Style},
        text::Span,
    };

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

        if contains_ignore_case(stripped, "error") || contains_ignore_case(stripped, "failed") {
            return tokenize_inline_code(line, Style::default().fg(theme.error), theme);
        }

        if contains_ignore_case(stripped, "success")
            || contains_ignore_case(stripped, "done")
            || stripped.contains("✓")
        {
            return tokenize_inline_code(line, Style::default().fg(theme.success), theme);
        }

        tokenize_inline_code(line, Style::default(), theme)
    }
}
mod normalize {
    pub(crate) fn normalize_session_detail_markdown(text: &str) -> String {
        let mut lines = text.lines().peekable();
        let Some(first_line) = lines.next() else {
            return text.to_string();
        };
        if lines.peek().is_none() {
            return text.to_string();
        }

        let mut out = String::with_capacity(text.len());
        let mut in_fenced_code = false;
        let mut line = first_line;
        let mut first = true;

        loop {
            if first {
                first = false;
            } else {
                out.push('\n');
            }
            out.push_str(line);

            let trimmed = line.trim();
            let can_insert_gap = if is_fence_marker(trimmed) {
                in_fenced_code = !in_fenced_code;
                false
            } else {
                !in_fenced_code
            };

            let Some(next) = lines.next() else {
                break;
            };
            if can_insert_gap && should_insert_session_paragraph_gap(line, next) {
                out.push('\n');
            }
            line = next;
        }

        out
    }

    fn should_insert_session_paragraph_gap(current: &str, next: &str) -> bool {
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

    fn is_fence_marker(line: &str) -> bool {
        line.starts_with("```") || line.starts_with("~~~")
    }

    fn is_setext_underline(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.len() >= 3 && trimmed.chars().all(|ch| matches!(ch, '-' | '='))
    }

    fn is_markdown_structural_line(line: &str) -> bool {
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
        if saw_digit && matches!(chars.next(), Some('.' | ')')) && matches!(chars.next(), Some(' '))
        {
            return true;
        }

        false
    }
}
mod render {
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
}
mod style {
    use super::super::common::{blend_color, fallback_color};
    use crate::theme::Theme;
    use ratatui::style::{Color, Modifier, Style};
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

    pub(crate) fn inline_code_style(theme: &Theme) -> Style {
        Style::default()
            .fg(derived_inline_code_fg(theme))
            .bg(derived_inline_code_bg(theme))
    }

    fn derived_inline_code_bg(theme: &Theme) -> Color {
        let base = fallback_color(theme.bg, theme.highlight_bg);
        let surface = fallback_color(theme.highlight_bg, theme.border);
        blend_color(surface, base, 0.72)
    }

    fn derived_inline_code_fg(theme: &Theme) -> Color {
        let base = fallback_color(theme.fg, theme.highlight_fg);
        let accent = fallback_color(theme.accent, base);
        blend_color(accent, base, 0.28)
    }
}
mod wrap;

pub(crate) use inline::{format_line, retokenize_inline_code, tokenize_inline_code};
pub(crate) use normalize::normalize_session_detail_markdown;
pub(crate) use render::{
    detail_surface, render_detail_content_line, render_detail_padding_line,
    render_detail_separator_line,
};
pub(crate) use style::markdown_options;
pub(crate) use wrap::{
    flatten_lines_for_smooth_scrolling, total_span_count, wrap_styled_line, wrap_text_to_width,
};

#[cfg(test)]
mod tests {
    mod inline {
        use super::super::inline::format_line;
        use crate::theme::Theme;

        #[test]
        fn format_line_detects_error_case_insensitively() {
            let theme = Theme::default();
            let spans = format_line("FAILED to run", &theme);

            assert_eq!(spans[0].style.fg, Some(theme.error));
        }

        #[test]
        fn format_line_detects_success_case_insensitively() {
            let theme = Theme::default();
            let spans = format_line("SUCCESS", &theme);

            assert_eq!(spans[0].style.fg, Some(theme.success));
        }
    }

    mod normalize {
        use super::super::normalize_session_detail_markdown;

        #[test]
        fn inserts_paragraph_gaps_between_plain_lines() {
            assert_eq!(
                normalize_session_detail_markdown("first\nsecond"),
                "first\n\nsecond"
            );
        }

        #[test]
        fn keeps_fenced_code_lines_together() {
            assert_eq!(
                normalize_session_detail_markdown("```rs\nlet a = 1;\nlet b = 2;\n```"),
                "```rs\nlet a = 1;\nlet b = 2;\n```"
            );
        }
    }
}
