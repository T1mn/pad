use crate::app::App;
use crate::log_debug;
use crate::tmux_bindings::{restore_binding_cmd, PAD_SIDER_TOGGLE_KEYS};

/// Clean up F12/C-q/F10/C-Tab root bindings and restore status bar — safety net for pad quit/crash.
pub(crate) fn restore_tmux_bindings(app: &mut App) {
    let trace_id = app
        .same_session_trace_id
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let saved_f12 = app
        .saved_tmux_bindings
        .iter()
        .find(|line| line.contains(" F12 "))
        .cloned();
    let saved_cq = app
        .saved_tmux_bindings
        .iter()
        .find(|line| line.contains(" C-q "))
        .cloned();

    let _ = std::process::Command::new("sh")
        .args(["-lc", &restore_binding_cmd(saved_f12.as_deref(), "F12")])
        .output();
    let _ = std::process::Command::new("sh")
        .args(["-lc", &restore_binding_cmd(saved_cq.as_deref(), "C-q")])
        .output();
    for key in PAD_SIDER_TOGGLE_KEYS {
        let saved_binding = app
            .saved_tmux_bindings
            .iter()
            .find(|line| line.contains(&format!(" {} ", key)))
            .cloned();
        let _ = std::process::Command::new("sh")
            .args(["-lc", &restore_binding_cmd(saved_binding.as_deref(), key)])
            .output();
    }

    if let (Some(status), Some(target)) = (
        app.saved_tmux_status.as_deref(),
        app.saved_tmux_status_target.as_deref(),
    ) {
        let _ = std::process::Command::new("tmux")
            .args(["set", "-t", target, "status", status])
            .output();
    }

    log_debug!(
        "handoff trace={} stage=restore_tmux_bindings restored root bindings and status",
        trace_id
    );
    app.saved_tmux_bindings.clear();
    app.saved_tmux_status = None;
    app.saved_tmux_status_target = None;
    app.same_session_trace_id = None;
}
