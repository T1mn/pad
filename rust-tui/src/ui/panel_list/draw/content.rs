use super::super::{empty, viewport};
use super::row::build_sidebar_row;
use crate::app::state::ThreadListView;
use crate::app::state::VisibleSidebarStats;
use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Rect},
    widgets::{Paragraph, Row, Table, Wrap},
    Frame,
};
use std::collections::HashSet;

pub(super) struct PanelListContentParams<'a> {
    pub(super) selected_idx: Option<usize>,
    pub(super) expanded_folders: &'a HashSet<String>,
    pub(super) hovered_folder_key: Option<&'a str>,
    pub(super) theme: &'a Theme,
    pub(super) visible_stats: VisibleSidebarStats,
}

#[derive(Clone, Copy)]
pub(super) struct PanelListRenderState {
    pub(super) show_scrollbar: bool,
    pub(super) actual_item_count: usize,
    pub(super) table_offset: usize,
}

pub(super) fn render_panel_list_content(
    f: &mut Frame,
    app: &mut App,
    inner: Rect,
    params: PanelListContentParams<'_>,
) -> PanelListRenderState {
    let locale = app.locale;
    let thread_list_view = app.thread_list_view();
    let selected_idx = params.selected_idx;
    let items = app.visible_sidebar_items_ref();
    let actual_item_count = params.visible_stats.item_count;
    let show_scrollbar = params.visible_stats.row_count > inner.height as usize;

    if items.is_empty() {
        render_empty_message(f, inner, locale, thread_list_view, params.theme);
        return PanelListRenderState {
            show_scrollbar,
            actual_item_count,
            table_offset: 0,
        };
    }

    let render_window =
        viewport::render_window(items.len(), selected_idx, inner.height as usize, |idx| {
            viewport::item_row_height(&items[idx])
        });
    let table_offset = render_window.start;
    let content_width = inner.width as usize;
    let mut next_jump_badge = viewport::next_jump_badge_for_start(items, render_window.start);
    let rows: Vec<Row> = items[render_window.clone()]
        .iter()
        .enumerate()
        .map(|(offset, item)| {
            let idx = render_window.start + offset;
            let jump_badge = viewport::jump_badge_for_item(item, &mut next_jump_badge);
            build_sidebar_row(
                item,
                jump_badge,
                idx == selected_idx.unwrap_or(usize::MAX),
                content_width,
                params.theme,
                params.expanded_folders.contains(item.folder_key()),
                params.hovered_folder_key == Some(item.folder_key()),
            )
        })
        .collect();

    let table = Table::new(rows, [Constraint::Min(0)])
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Never);
    let mut table_state = ratatui::widgets::TableState::default();
    table_state.select(
        selected_idx
            .and_then(|idx| idx.checked_sub(render_window.start))
            .filter(|idx| *idx < render_window.len()),
    );
    f.render_stateful_widget(table, inner, &mut table_state);

    PanelListRenderState {
        show_scrollbar,
        actual_item_count,
        table_offset,
    }
}

fn render_empty_message(
    f: &mut Frame,
    inner: Rect,
    locale: crate::i18n::Locale,
    thread_list_view: ThreadListView,
    theme: &Theme,
) {
    let empty = Paragraph::new(empty::empty_message(locale, thread_list_view, theme))
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: false });
    f.render_widget(empty, inner);
}
