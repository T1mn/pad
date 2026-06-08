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
