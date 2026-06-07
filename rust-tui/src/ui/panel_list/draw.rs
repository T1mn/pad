use super::labels::{display_scope_title_label, special_view_title_label};
use super::{empty, folder_row, thread_row, viewport};
use crate::app::state::{FocusTarget, ThreadListView};
use crate::app::App;
use crate::sidebar::SidebarItem;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::Style,
    widgets::{
        Block, BorderType, Borders, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, Wrap,
    },
    Frame,
};

pub fn draw_panel_list(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme.clone();
    let l = app.locale;
    let thread_list_view = app.thread_list_view();
    let showing_live = app.showing_live_sessions();
    let panel_is_focused = !app.sidebar.show_tree && app.preview.focus == FocusTarget::Panel;
    let selected_idx = app.table_state.selected();
    let expanded_folders = app.sidebar.expanded_folders.clone();
    let hovered_folder_key = app.sidebar.hovered_folder_key.clone();
    let visible_stats = {
        app.visible_sidebar_items_ref();
        app.sidebar.visible_sidebar_stats
    };

    let item_count = visible_stats.thread_count;
    let border_color = if panel_is_focused {
        theme.border_focused
    } else {
        theme.border
    };
    let focus_mark = if panel_is_focused { "●" } else { "○" };
    let title = if thread_list_view != ThreadListView::Normal {
        format!(
            " {} {} {} ",
            focus_mark,
            special_view_title_label(l, thread_list_view),
            item_count
        )
    } else {
        format!(
            " {} {} {} ",
            focus_mark,
            display_scope_title_label(l, showing_live),
            item_count
        )
    };
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let (show_scrollbar, actual_item_count, table_offset) = {
        let items = app.visible_sidebar_items_ref();
        let actual_item_count = visible_stats.item_count;
        let content_width = inner.width as usize;
        let total_h = visible_stats.row_count;
        let show_scrollbar = total_h > inner.height as usize;
        let mut table_offset = 0usize;

        if items.is_empty() {
            let empty = Paragraph::new(empty::empty_message(l, thread_list_view, &theme))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });
            f.render_widget(empty, inner);
        } else {
            let render_window =
                viewport::render_window(items.len(), selected_idx, inner.height as usize, |idx| {
                    viewport::item_row_height(&items[idx])
                });
            table_offset = render_window.start;
            let mut next_jump_badge =
                viewport::next_jump_badge_for_start(items, render_window.start);
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
                        &theme,
                        expanded_folders.contains(item.folder_key()),
                        hovered_folder_key.as_deref() == Some(item.folder_key()),
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
        }
        (show_scrollbar, actual_item_count, table_offset)
    };
    *app.table_state.offset_mut() = table_offset;

    if show_scrollbar && actual_item_count > 0 {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state =
            ScrollbarState::new(actual_item_count).position(selected_idx.unwrap_or(0));
        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn build_sidebar_row(
    item: &SidebarItem,
    jump_badge: Option<usize>,
    is_selected: bool,
    content_width: usize,
    theme: &crate::theme::Theme,
    is_expanded: bool,
    is_hovered_folder: bool,
) -> Row<'static> {
    match item {
        SidebarItem::Folder(folder) => folder_row::build_folder_row(
            folder,
            is_selected,
            content_width,
            theme,
            is_expanded,
            is_hovered_folder,
        ),
        SidebarItem::Thread(thread) => {
            thread_row::build_thread_row(thread, is_selected, content_width, theme, jump_badge)
        }
    }
}
