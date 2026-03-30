#![allow(dead_code)]

use crate::theme::Theme;
use crate::ui::selection::{SelectionItem, SelectionState};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState, Wrap};
use ratatui::Frame;

pub const MIN_SIDE_BY_SIDE_WIDTH: u16 = 72;
const SELECTION_SPLIT_GAP: u16 = 2;
const SELECTION_HORIZONTAL_PADDING: u16 = 2;
const SELECTION_VERTICAL_PADDING: u16 = 1;

pub const fn selection_block_vertical_overhead() -> u16 {
    2 + SELECTION_VERTICAL_PADDING * 2
}

pub const fn selection_surface_padding_height() -> u16 {
    SELECTION_VERTICAL_PADDING * 2
}

pub fn recommended_split_modal_height(
    width: u16,
    item_count: u16,
    row_height: u16,
    detail_lines: u16,
) -> u16 {
    let list_height = item_count
        .saturating_mul(row_height)
        .saturating_add(selection_block_vertical_overhead());
    let detail_height = detail_lines.saturating_add(selection_block_vertical_overhead());
    if width >= MIN_SIDE_BY_SIDE_WIDTH {
        list_height.max(detail_height)
    } else {
        list_height
            .saturating_add(SELECTION_SPLIT_GAP)
            .saturating_add(detail_height)
    }
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
    render_selection_list_rows(f, sections[1], theme, title, items, state, true);
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

pub fn render_selection_split(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    items: &[SelectionItem],
    state: &SelectionState,
) {
    let shared_inner = padded_inner(area);
    let sections = if area.width >= MIN_SIDE_BY_SIDE_WIDTH {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(41),
                Constraint::Length(SELECTION_SPLIT_GAP),
                Constraint::Percentage(59),
            ])
            .split(shared_inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(44),
                Constraint::Length(SELECTION_SPLIT_GAP),
                Constraint::Percentage(56),
            ])
            .split(shared_inner)
    };

    render_selection_list_content(f, sections[0], theme, title, items, state);
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.bg).fg(theme.comment)),
        sections[1],
    );
    let filtered = state.filtered_indices(items, SelectionItem::matches_query);
    let selected = state
        .selected_filtered_index(items, SelectionItem::matches_query)
        .and_then(|idx| filtered.get(idx).copied())
        .and_then(|idx| items.get(idx));
    render_selection_detail_content(f, sections[2], theme, "Detail", selected);
}

pub fn render_selection_list(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    items: &[SelectionItem],
    state: &SelectionState,
) {
    let list_title = if state.searching || !state.query.is_empty() {
        format!("{}  /{}", title, state.query)
    } else {
        title.to_string()
    };
    let block = Block::default()
        .title(format!("  {}  ", list_title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));
    let inner = padded_inner(block.inner(area));
    f.render_widget(block, area);

    render_selection_list_rows(f, inner, theme, title, items, state, false);
}

fn render_selection_list_content(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    items: &[SelectionItem],
    state: &SelectionState,
) {
    let [title_area, content_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let list_title = if state.searching || !state.query.is_empty() {
        format!("{} /{}", title, state.query)
    } else {
        title.to_string()
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            list_title,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        title_area,
    );
    render_selection_list_rows(f, content_area, theme, title, items, state, true);
}

fn render_selection_list_rows(
    f: &mut Frame,
    inner: Rect,
    theme: &Theme,
    _title: &str,
    items: &[SelectionItem],
    state: &SelectionState,
    compact: bool,
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
            let mut lines = vec![Line::from(Span::styled(item.title.clone(), title_style))];
            if let Some(subtitle) = item.subtitle.as_ref() {
                let subtitle_style = if item.disabled {
                    Style::default()
                        .fg(theme.comment)
                        .bg(row_bg)
                        .add_modifier(Modifier::DIM)
                } else if is_selected {
                    Style::default().fg(theme.highlight_fg).bg(row_bg)
                } else {
                    Style::default()
                        .fg(theme.comment)
                        .bg(row_bg)
                        .add_modifier(Modifier::DIM)
                };
                lines.push(Line::from(Span::styled(subtitle.clone(), subtitle_style)));
            }
            let row = Row::new(vec![Cell::from(lines)])
                .height(if item.subtitle.is_some() { 2 } else { 1 })
                .style(Style::default().bg(row_bg));
            let _ = compact;
            row
        })
        .collect();

    let table = Table::new(rows, [Constraint::Min(0)])
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Never);
    let mut table_state = TableState::default().with_selected(Some(selected));
    f.render_stateful_widget(table, inner, &mut table_state);
}

fn render_selection_detail(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    selected: Option<&SelectionItem>,
) {
    let block = Block::default()
        .title("  Detail  ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));
    let inner = padded_inner(block.inner(area));
    f.render_widget(block, area);

    render_selection_detail_body(f, inner, theme, selected);
}

fn render_selection_detail_content(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    selected: Option<&SelectionItem>,
) {
    let [title_area, content_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title.to_string(),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        title_area,
    );
    render_selection_detail_body(f, content_area, theme, selected);
}

fn render_selection_detail_body(
    f: &mut Frame,
    inner: Rect,
    theme: &Theme,
    selected: Option<&SelectionItem>,
) {
    let Some(item) = selected else {
        f.render_widget(
            Paragraph::new("No selection")
                .style(Style::default().fg(theme.comment))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    };

    let mut lines = Vec::new();
    if let Some(detail) = item.detail.as_ref() {
        if let Some(title) = detail.title.as_ref() {
            lines.push(Line::from(Span::styled(
                title.clone(),
                Style::default()
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::default());
        }
        lines.extend(detail.body.clone());
    } else {
        lines.push(Line::from(Span::styled(
            item.title.clone(),
            Style::default()
                .fg(theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )));
        if let Some(subtitle) = item.subtitle.as_ref() {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                subtitle.clone(),
                Style::default().fg(theme.comment),
            )));
        }
    }

    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(theme.fg))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn padded_inner(area: Rect) -> Rect {
    let horizontal = SELECTION_HORIZONTAL_PADDING.min(area.width.saturating_sub(1) / 2);
    let vertical = SELECTION_VERTICAL_PADDING.min(area.height.saturating_sub(1) / 2);
    area.inner(Margin {
        horizontal,
        vertical,
    })
}
