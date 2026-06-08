use crate::theme::Theme;
use crate::ui::selection::SelectionItem;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

mod style;
mod width;

#[cfg(test)]
mod tests;

use style::{marker_style, value_style};
use width::{display_width, truncate_to_width};

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
