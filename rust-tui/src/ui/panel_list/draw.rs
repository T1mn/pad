mod block;
mod content;
mod row;
mod scrollbar;

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
