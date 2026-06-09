use crate::theme::Theme;
use crate::ui::selection::row::render_selection_title_line;
use crate::ui::selection::{SelectionItem, SelectionState};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table, TableState, Wrap};
use ratatui::Frame;

pub(super) fn render_selection_list_rows(
    f: &mut Frame,
    inner: Rect,
    theme: &Theme,
    items: &[SelectionItem],
    state: &SelectionState,
) {
    let filtered = state.filtered_indices(items, SelectionItem::matches_query);
    if filtered.is_empty() {
        f.render_widget(
            Paragraph::new("No matches")
                .style(Style::default().fg(theme.comment))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let selected = state.selected.min(filtered.len().saturating_sub(1));
    let rows = filtered
        .iter()
        .enumerate()
        .filter_map(|(visible_idx, &idx)| items.get(idx).map(|item| (visible_idx, item)))
        .map(|(visible_idx, item)| render_selection_row(visible_idx, selected, item, inner, theme))
        .collect::<Vec<_>>();

    let table = Table::new(rows, [Constraint::Min(0)])
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Never);
    let mut table_state = TableState::default().with_selected(Some(selected));
    f.render_stateful_widget(table, inner, &mut table_state);
}

fn render_selection_row(
    visible_idx: usize,
    selected: usize,
    item: &SelectionItem,
    inner: Rect,
    theme: &Theme,
) -> Row<'static> {
    let is_selected = visible_idx == selected;
    let row_bg = if is_selected {
        theme.highlight_bg
    } else {
        theme.bg
    };
    let title_style = title_style(item, theme, row_bg, is_selected);
    let mut lines = vec![render_selection_title_line(
        item,
        inner.width,
        theme,
        row_bg,
        is_selected,
        title_style,
    )];
    if let Some(subtitle) = item.subtitle.as_ref() {
        lines.push(render_subtitle_line(
            subtitle,
            item,
            theme,
            row_bg,
            is_selected,
        ));
    }
    Row::new(vec![Cell::from(lines)])
        .height(if item.subtitle.is_some() { 2 } else { 1 })
        .style(Style::default().bg(row_bg))
}

fn title_style(
    item: &SelectionItem,
    theme: &Theme,
    row_bg: ratatui::style::Color,
    is_selected: bool,
) -> Style {
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
        Style::default()
            .fg(theme.fg)
            .bg(row_bg)
            .add_modifier(Modifier::BOLD)
    }
}

fn render_subtitle_line(
    subtitle: &str,
    item: &SelectionItem,
    theme: &Theme,
    row_bg: ratatui::style::Color,
    is_selected: bool,
) -> Line<'static> {
    let subtitle_style = if item.disabled {
        Style::default()
            .fg(theme.comment)
            .bg(row_bg)
            .add_modifier(Modifier::DIM)
    } else if is_selected {
        Style::default().fg(theme.highlight_fg).bg(row_bg)
    } else {
        Style::default().fg(theme.comment).bg(row_bg)
    };
    Line::from(vec![
        Span::styled("  ", Style::default().bg(row_bg)),
        Span::styled(subtitle.to_string(), subtitle_style),
    ])
}
