use crate::theme::Theme;
use crate::ui::selection::row::render_selection_title_line;
use crate::ui::selection::{SelectionItem, SelectionState};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table, TableState, Wrap};
use ratatui::Frame;

const SELECTION_HORIZONTAL_PADDING: u16 = 2;
const SELECTION_VERTICAL_PADDING: u16 = 1;

pub const fn selection_surface_padding_height() -> u16 {
    SELECTION_VERTICAL_PADDING * 2
}

pub fn recommended_list_modal_height(
    item_count: u16,
    row_height: u16,
    header_lines: u16,
    footer_lines: u16,
) -> u16 {
    item_count
        .max(1)
        .saturating_mul(row_height)
        .saturating_add(header_lines)
        .saturating_add(footer_lines)
        .saturating_add(selection_surface_padding_height())
}

pub fn render_selection_surface(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    items: &[SelectionItem],
    state: &SelectionState,
    footer: Option<&str>,
) {
    let inner = padded_inner(area);
    let mut constraints = vec![Constraint::Length(1), Constraint::Min(0)];
    if footer.is_some() {
        constraints.push(Constraint::Length(1));
    }
    let sections = Layout::vertical(constraints).split(inner);
    let header = if state.searching || !state.query.is_empty() {
        if state.searching {
            format!("/{}|", state.query)
        } else {
            format!("/{}", state.query)
        }
    } else {
        title.to_string()
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            header,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        sections[0],
    );
    render_selection_list_rows(f, sections[1], theme, items, state);
    if let Some(footer_text) = footer {
        if let Some(footer_area) = sections.get(2) {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    footer_text.to_string(),
                    Style::default()
                        .fg(theme.comment)
                        .add_modifier(Modifier::DIM),
                ))),
                *footer_area,
            );
        }
    }
}

fn render_selection_list_rows(
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

    let selected = state
        .selected_filtered_index(items, SelectionItem::matches_query)
        .unwrap_or(0);
    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .filter_map(|(visible_idx, &idx)| items.get(idx).map(|item| (visible_idx, item)))
        .map(|(visible_idx, item)| {
            let is_selected = visible_idx == selected;
            let row_bg = if is_selected {
                theme.highlight_bg
            } else {
                theme.bg
            };
            let title_style = if item.disabled {
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
            };
            let mut lines = vec![render_selection_title_line(
                item,
                inner.width,
                theme,
                row_bg,
                is_selected,
                title_style,
            )];
            if let Some(subtitle) = item.subtitle.as_ref() {
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
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().bg(row_bg)),
                    Span::styled(subtitle.clone(), subtitle_style),
                ]));
            }
            Row::new(vec![Cell::from(lines)])
                .height(if item.subtitle.is_some() { 2 } else { 1 })
                .style(Style::default().bg(row_bg))
        })
        .collect();

    let table = Table::new(rows, [Constraint::Min(0)])
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Never);
    let mut table_state = TableState::default().with_selected(Some(selected));
    f.render_stateful_widget(table, inner, &mut table_state);
}

fn padded_inner(area: Rect) -> Rect {
    let horizontal = SELECTION_HORIZONTAL_PADDING.min(area.width.saturating_sub(1) / 2);
    let vertical = SELECTION_VERTICAL_PADDING.min(area.height.saturating_sub(1) / 2);
    area.inner(Margin {
        horizontal,
        vertical,
    })
}
