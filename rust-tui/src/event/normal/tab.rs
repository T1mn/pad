use crate::app::App;
use std::time::Duration;

pub(super) fn handle_preview_tab(app: &mut App) {
    const DOUBLE_TAB_WINDOW: Duration = Duration::from_millis(350);

    if app.preview_is_focused() {
        if app.preview.view == crate::model::PreviewView::SessionDetail {
            app.note_detail_exit_tab();
            app.toggle_preview_focus();
            app.clear_panel_tab();
            return;
        }
        if app.recent_panel_tab_within(DOUBLE_TAB_WINDOW) && app.open_latest_preview_turn() {
            app.clear_panel_tab();
            return;
        }
        app.toggle_preview_focus();
        app.clear_panel_tab();
        return;
    }

    if app.recent_detail_exit_tab_within(DOUBLE_TAB_WINDOW)
        && app.selected_thread_matches_preview_target()
        && app.restore_preview_turns_list()
    {
        app.focus_panel();
        app.clear_detail_exit_tab();
        app.clear_panel_tab();
        return;
    }

    if app.toggle_preview_focus() {
        app.note_panel_tab();
        app.clear_detail_exit_tab();
    } else {
        app.clear_panel_tab();
        app.clear_detail_exit_tab();
    }
}
