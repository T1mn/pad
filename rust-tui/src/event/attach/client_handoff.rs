mod logging;
mod steps;

use crate::app::App;
use crate::model::AgentPanel;

use super::bindings::install_return_bindings;

pub(super) fn handoff_to_tmux_client(
    app: &mut App,
    panel: &AgentPanel,
    current_session: &str,
    cross_session: bool,
) -> bool {
    let target_window = format!("{}:{}", panel.session, panel.window_index);
    let prefix = if cross_session {
        "attach.cross_session"
    } else {
        "attach.same_session"
    };

    logging::log_start(prefix, &target_window, panel, current_session);
    let should_zoom = install_return_bindings(app, &panel.pane_id, &panel.session);

    if cross_session {
        if !steps::switch_client(app, &panel.session) {
            return false;
        }
        logging::log_after_step(prefix, "after switch-client", &panel.pane_id);
    }

    if !steps::select_window(app, prefix, &target_window) {
        return false;
    }
    logging::log_after_step(prefix, "after select-window", &panel.pane_id);

    if !steps::select_pane(app, prefix, &panel.pane_id) {
        return false;
    }
    logging::log_after_step(prefix, "after select-pane", &panel.pane_id);

    if should_zoom {
        if !steps::resize_zoom(app, prefix, &panel.pane_id) {
            return false;
        }
        logging::log_after_step(prefix, "after resize-pane", &panel.pane_id);
    }

    app.same_session_attached = true;
    logging::log_complete(prefix, &target_window, panel, should_zoom);
    app.dirty = true;
    true
}
