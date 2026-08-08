mod block {
    use super::super::labels::{display_scope_title_label, special_view_title_label};
    use crate::app::state::ThreadListView;
    use crate::i18n::Locale;
    use crate::theme::Theme;
    use ratatui::{
        layout::Alignment,
        style::Style,
        widgets::{Block, BorderType, Borders},
    };

    pub(super) fn panel_list_block(
        locale: Locale,
        thread_list_view: ThreadListView,
        showing_live: bool,
        panel_is_focused: bool,
        item_count: usize,
        theme: &Theme,
    ) -> Block<'static> {
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
                special_view_title_label(locale, thread_list_view),
                item_count
            )
        } else {
            format!(
                " {} {} {} ",
                focus_mark,
                display_scope_title_label(locale, showing_live),
                item_count
            )
        };

        Block::default()
            .title(title)
            .title_alignment(Alignment::Left)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(theme.bg).fg(theme.fg))
            .border_style(Style::default().fg(border_color))
    }
}
mod content {
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
}
mod row {
    use super::super::{folder_row, thread_row};
    use crate::sidebar::SidebarItem;
    use ratatui::widgets::Row;

    pub(super) fn build_sidebar_row(
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
}
mod scrollbar {
    use super::content::PanelListRenderState;
    use ratatui::{
        layout::{Margin, Rect},
        widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
        Frame,
    };

    pub(super) fn render_panel_scrollbar(
        f: &mut Frame,
        area: Rect,
        selected_idx: Option<usize>,
        render_state: PanelListRenderState,
    ) {
        if !render_state.show_scrollbar || render_state.actual_item_count == 0 {
            return;
        }

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state =
            ScrollbarState::new(render_state.actual_item_count).position(selected_idx.unwrap_or(0));
        f.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

use crate::app::state::FocusTarget;
use crate::app::App;
use ratatui::{layout::Rect, Frame};

pub fn draw_panel_list(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme.clone();
    let locale = app.locale;
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

    let block = block::panel_list_block(
        locale,
        thread_list_view,
        showing_live,
        panel_is_focused,
        visible_stats.thread_count,
        &theme,
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let render_state = content::render_panel_list_content(
        f,
        app,
        inner,
        content::PanelListContentParams {
            selected_idx,
            expanded_folders: &expanded_folders,
            hovered_folder_key: hovered_folder_key.as_deref(),
            theme: &theme,
            visible_stats,
        },
    );
    *app.table_state.offset_mut() = render_state.table_offset;

    scrollbar::render_panel_scrollbar(f, area, selected_idx, render_state);
}
