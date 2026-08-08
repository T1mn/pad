mod block;
mod content;
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
