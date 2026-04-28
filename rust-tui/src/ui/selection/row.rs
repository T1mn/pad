use crate::theme::Theme;
use crate::ui::selection::SelectionItem;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub(super) fn render_selection_title_line(
    item: &SelectionItem,
    width: u16,
    theme: &Theme,
    row_bg: Color,
    is_selected: bool,
    title_style: Style,
) -> Line<'static> {
    let marker = if is_selected { "❯ " } else { "  " };
    let marker_style = marker_style(item, theme, row_bg, is_selected);
    let mut spans = vec![
        Span::styled(marker.to_string(), marker_style),
        Span::styled(item.title.clone(), title_style),
    ];

    if let Some(value) = item.value.as_ref() {
        let fixed_width = display_width(marker) + display_width(&item.title) + 1;
        let value = truncate_to_width(value, (width as usize).saturating_sub(fixed_width));
        let used_width = display_width(marker) + display_width(&item.title) + display_width(&value);
        let gap = (width as usize).saturating_sub(used_width).max(1);
        spans.push(Span::styled(" ".repeat(gap), Style::default().bg(row_bg)));
        spans.push(Span::styled(
            value,
            value_style(item, theme, row_bg, is_selected),
        ));
    }

    Line::from(spans)
}

fn marker_style(item: &SelectionItem, theme: &Theme, row_bg: Color, is_selected: bool) -> Style {
    if item.disabled {
        Style::default()
            .fg(theme.comment)
            .bg(row_bg)
            .add_modifier(Modifier::DIM)
    } else if is_selected {
        Style::default()
            .fg(theme.border_focused)
            .bg(row_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(row_bg)
    }
}

fn value_style(item: &SelectionItem, theme: &Theme, row_bg: Color, is_selected: bool) -> Style {
    if item.disabled {
        Style::default()
            .fg(theme.comment)
            .bg(row_bg)
            .add_modifier(Modifier::DIM)
    } else if is_selected {
        Style::default()
            .fg(theme.highlight_fg)
            .bg(row_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.accent).bg(row_bg)
    }
}

fn display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let ellipsis = "…";
    let target_width = max_width.saturating_sub(display_width(ellipsis));
    let mut out = String::new();
    let mut used = 0usize;

    for ch in text.chars() {
        let width = char_display_width(ch);
        if used + width > target_width {
            break;
        }
        out.push(ch);
        used += width;
    }

    out.push_str(ellipsis);
    out
}

fn char_display_width(c: char) -> usize {
    if c == '\t' {
        return 4;
    }
    if c.is_control() {
        return 0;
    }

    let code = c as u32;
    if matches!(
        code,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x3FFFD
    ) {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::render_selection_title_line;
    use crate::theme::Theme;
    use crate::ui::selection::SelectionItem;
    use ratatui::style::Style;

    #[test]
    fn title_line_marks_selection_and_right_aligns_value() {
        let theme = Theme::default();
        let item = SelectionItem {
            title: "Theme".into(),
            value: Some("dark  ›".into()),
            ..Default::default()
        };

        let line = render_selection_title_line(&item, 20, &theme, theme.bg, true, Style::default());

        assert_eq!(line.spans[0].content.as_ref(), "❯ ");
        assert_eq!(line.spans[1].content.as_ref(), "Theme");
        assert_eq!(line.spans[3].content.as_ref(), "dark  ›");
        assert_eq!(line.spans[3].style.fg, Some(theme.highlight_fg));
    }

    #[test]
    fn unselected_value_uses_accent_color() {
        let theme = Theme::default();
        let item = SelectionItem {
            title: "Sound".into(),
            value: Some("on  ›".into()),
            ..Default::default()
        };

        let line =
            render_selection_title_line(&item, 20, &theme, theme.bg, false, Style::default());

        assert_eq!(line.spans[0].content.as_ref(), "  ");
        assert_eq!(line.spans[3].style.fg, Some(theme.accent));
    }
}
