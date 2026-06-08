use crate::app::App;
use crate::log_debug;

use super::super::bindings::restore_tmux_bindings;
use super::super::tmux::run_tmux_success;

pub(super) fn switch_client(app: &mut App, target_session: &str) -> bool {
    run_or_restore(
        app,
        "attach.cross_session.switch_client",
        vec![
            "switch-client".to_string(),
            "-t".to_string(),
            target_session.to_string(),
        ],
        || {
            log_debug!(
                "attach.cross_session: switch-client failed target_session={}",
                target_session
            );
        },
    )
}

pub(super) fn select_window(app: &mut App, prefix: &str, target_window: &str) -> bool {
    run_or_restore(
        app,
        &format!("{}.select_window", prefix),
        vec![
            "select-window".to_string(),
            "-t".to_string(),
            target_window.to_string(),
        ],
        || {
            log_debug!(
                "{}: select-window failed target_window={}",
                prefix,
                target_window
            );
        },
    )
}

pub(super) fn select_pane(app: &mut App, prefix: &str, target_pane: &str) -> bool {
    run_or_restore(
        app,
        &format!("{}.select_pane", prefix),
        vec![
            "select-pane".to_string(),
            "-t".to_string(),
            target_pane.to_string(),
        ],
        || {
            log_debug!("{}: select-pane failed target_pane={}", prefix, target_pane);
        },
    )
}

pub(super) fn resize_zoom(app: &mut App, prefix: &str, target_pane: &str) -> bool {
    run_or_restore(
        app,
        &format!("{}.resize_zoom", prefix),
        vec![
            "resize-pane".to_string(),
            "-Z".to_string(),
            "-t".to_string(),
            target_pane.to_string(),
        ],
        || {
            log_debug!("{}: resize-pane failed target_pane={}", prefix, target_pane);
        },
    )
}

fn run_or_restore(
    app: &mut App,
    label: &str,
    args: Vec<String>,
    log_failure: impl FnOnce(),
) -> bool {
    if run_tmux_success(label, args) {
        return true;
    }

    log_failure();
    restore_tmux_bindings(app);
    false
}
