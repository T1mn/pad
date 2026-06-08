use crate::app::App;
use crate::model::{PreviewSessionOrigin, PreviewSource, PreviewView, SessionCacheState};
use crate::preview_source::PreviewUpdate;

pub(super) struct PreviewUpdateSnapshot {
    pub(super) previous_panel_cache_state: Option<SessionCacheState>,
    previous_pane_id: Option<String>,
    previous_source: PreviewSource,
    previous_view: PreviewView,
    previous_session_origin: Option<PreviewSessionOrigin>,
    previous_session_id: Option<String>,
    previous_selected_turn: Option<usize>,
    previous_expanded_turn: Option<usize>,
    previous_list_scroll: u16,
    previous_detail_scroll: u16,
    previous_follow_bottom: bool,
    previous_follow_selection: bool,
}

impl PreviewUpdateSnapshot {
    pub(super) fn capture(app: &App, update: &PreviewUpdate) -> Self {
        Self {
            previous_panel_cache_state: app
                .panels
                .iter()
                .find(|panel| update.live_pane_id.as_deref() == Some(panel.pane_id.as_str()))
                .and_then(|panel| panel.session_cache_state),
            previous_pane_id: app.preview.pane_id.clone(),
            previous_source: app.preview.source,
            previous_view: app.preview.view,
            previous_session_origin: app.preview.session_origin,
            previous_session_id: app.preview.session_id.clone(),
            previous_selected_turn: app.preview.selected_turn,
            previous_expanded_turn: app.preview.expanded_turn,
            previous_list_scroll: app.preview.list_scroll,
            previous_detail_scroll: app.preview.detail_scroll,
            previous_follow_bottom: app.preview.follow_bottom,
            previous_follow_selection: app.preview.follow_selection,
        }
    }

    pub(super) fn preview_state_changed(
        &self,
        app: &App,
        content_changed: bool,
        turns_changed: bool,
        panel_cache_state_changed: bool,
    ) -> bool {
        self.previous_pane_id != app.preview.pane_id
            || self.previous_source != app.preview.source
            || self.previous_view != app.preview.view
            || self.previous_session_origin != app.preview.session_origin
            || self.previous_session_id != app.preview.session_id
            || content_changed
            || turns_changed
            || self.previous_selected_turn != app.preview.selected_turn
            || self.previous_expanded_turn != app.preview.expanded_turn
            || self.previous_list_scroll != app.preview.list_scroll
            || self.previous_detail_scroll != app.preview.detail_scroll
            || self.previous_follow_bottom != app.preview.follow_bottom
            || self.previous_follow_selection != app.preview.follow_selection
            || panel_cache_state_changed
    }
}
