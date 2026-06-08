use crate::app::App;
use crate::log_debug;

use super::tmux::{
    apply_desired_status, run_tmux_logged, shell_log_cmd, summarize_log_text, tmux_status_value,
    wait_for_zoom_flag_cmd, wrap_tmux_run_shell,
};
use crate::tmux_bindings::{
    current_root_binding, pad_sider_toggle_command, PAD_RETURN_BINDING_MARKER,
};
pub(super) use crate::tmux_bindings::{restore_binding_cmd, PAD_SIDER_TOGGLE_KEYS};

/// Install F12/C-q/F10/C-Tab bindings for same-session attach.
/// Snapshots zoom and status bar state, modifies them for the attach,
/// and encodes restoration into the return command.
pub(super) fn install_return_bindings(
    app: &mut App,
    target_pane_id: &str,
    target_session: &str,
) -> bool {
    let trace_id = app
        .same_session_trace_id
        .clone()
        .unwrap_or_else(|| crate::app::new_handoff_trace("attach"));
    app.same_session_trace_id = Some(trace_id.clone());
    let pad_pane_id = match std::env::var("TMUX_PANE") {
        Ok(id) => id,
        Err(_) => {
            log_debug!(
                "handoff trace={} stage=attach.skip reason=tmux_pane_missing",
                trace_id
            );
            return false;
        }
    };

    log_debug!(
        "handoff trace={} stage=attach.begin target_pane={} target_session={} pad_pane={}",
        trace_id,
        target_pane_id,
        target_session,
        pad_pane_id
    );

    // Get pad's session:window_index for cross-window return
    let pad_win_target = std::process::Command::new("tmux")
        .args([
            "display-message",
            "-t",
            &pad_pane_id,
            "-p",
            "#{session_name}:#{window_index}",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if pad_win_target.is_empty() {
        log_debug!(
            "install_return_bindings: pad_win_target empty, pad_pane_id={}",
            pad_pane_id
        );
        return false;
    }

    let pad_session = std::process::Command::new("tmux")
        .args([
            "display-message",
            "-t",
            &pad_pane_id,
            "-p",
            "#{session_name}",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if pad_session.is_empty() {
        log_debug!(
            "install_return_bindings: pad_session empty, pad_pane_id={}",
            pad_pane_id
        );
        return false;
    }

    // --- Zoom: respect desired_agent_style.zoom config ---
    let zoom_info = std::process::Command::new("tmux")
        .args([
            "display-message",
            "-t",
            target_pane_id,
            "-p",
            "#{window_zoomed_flag} #{window_panes}",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let mut parts = zoom_info.split_whitespace();
    let already_zoomed = parts.next().unwrap_or("0") == "1";
    let pane_count: usize = parts.next().unwrap_or("1").parse().unwrap_or(1);
    let want_zoom = app.config.desired_agent_style.zoom == "auto";
    let should_zoom = want_zoom && pane_count > 1 && !already_zoomed;

    let restore_zoom_cmd = if should_zoom {
        // Do NOT zoom here — zoom happens after select-pane so user sees it instantly
        format!("tmux resize-pane -Z -t '{}'", target_pane_id)
    } else {
        String::new()
    };

    let saved_f12 = current_root_binding("F12");
    let saved_cq = current_root_binding("C-q");
    let saved_sider_bindings = PAD_SIDER_TOGGLE_KEYS
        .iter()
        .map(|key| (*key, current_root_binding(key)))
        .collect::<Vec<_>>();
    app.saved_tmux_bindings.clear();
    if let Some(line) = &saved_f12 {
        app.saved_tmux_bindings.push(line.clone());
    }
    if let Some(line) = &saved_cq {
        app.saved_tmux_bindings.push(line.clone());
    }
    for (_, saved_binding) in &saved_sider_bindings {
        if let Some(line) = saved_binding {
            app.saved_tmux_bindings.push(line.clone());
        }
    }

    log_debug!(
        "install_return_bindings: saved_bindings f12={} cq={}",
        saved_f12
            .as_deref()
            .map(summarize_log_text)
            .unwrap_or_else(|| "-".to_string()),
        saved_cq
            .as_deref()
            .map(summarize_log_text)
            .unwrap_or_else(|| "-".to_string())
    );

    // --- Status bar: respect desired_agent_style.status config ---
    let status_val = tmux_status_value(Some(target_session));
    let desired_status = app.config.desired_agent_style.status.as_str();
    let status_restore_value = apply_desired_status(desired_status, &status_val, target_session);
    app.saved_tmux_status = status_restore_value.clone();
    app.saved_tmux_status_target = status_restore_value
        .as_ref()
        .map(|_| target_session.to_string());
    let restore_status_cmd = status_restore_value
        .as_ref()
        .map(|status| format!("tmux set -t '{}' status '{}'", target_session, status))
        .unwrap_or_default();

    log_debug!(
        "install_return_bindings: target={} target_session={} panes={} zoomed={} should_zoom={} status={} desired_status={} status_restore={} pad_session={} pad_win={}",
        target_pane_id,
        target_session,
        pane_count,
        already_zoomed,
        should_zoom,
        status_val,
        desired_status,
        status_restore_value.as_deref().unwrap_or("-"),
        pad_session,
        pad_win_target
    );

    // Build return command: restore zoom + status, then navigate back to pad
    let mut restore_parts: Vec<String> = Vec::new();
    restore_parts.push(shell_log_cmd(&format!(
        "[handoff trace={}] stage=return.begin target_session={} target_pane={} pad_session={} pad_window={} pad_pane={}",
        trace_id, target_session, target_pane_id, pad_session, pad_win_target, pad_pane_id
    )));
    restore_parts.push(restore_binding_cmd(saved_f12.as_deref(), "F12"));
    restore_parts.push(restore_binding_cmd(saved_cq.as_deref(), "C-q"));
    for (key, saved_binding) in &saved_sider_bindings {
        restore_parts.push(restore_binding_cmd(saved_binding.as_deref(), key));
    }
    if !restore_zoom_cmd.is_empty() {
        restore_parts.push(shell_log_cmd(&format!(
            "before_unzoom target_pane={}",
            target_pane_id
        )));
        restore_parts.push(restore_zoom_cmd);
        restore_parts.push(shell_log_cmd(&format!(
            "after_unzoom target_pane={}",
            target_pane_id
        )));
        restore_parts.push(wait_for_zoom_flag_cmd(
            target_pane_id,
            "0",
            "after_unzoom_wait",
        ));
    }
    if !restore_status_cmd.is_empty() {
        restore_parts.push(restore_status_cmd);
    }
    if target_session != pad_session {
        restore_parts.push(shell_log_cmd(&format!(
            "before_return_switch target_session={} pad_session={}",
            target_session, pad_session
        )));
        restore_parts.push(format!("tmux switch-client -t '{}'", pad_session));
        restore_parts.push(shell_log_cmd(&format!(
            "after_return_switch target_session={} pad_session={}",
            target_session, pad_session
        )));
    }
    restore_parts.push(shell_log_cmd(&format!(
        "before_return_select pad_window={} pad_pane={}",
        pad_win_target, pad_pane_id
    )));
    restore_parts.push(format!("tmux select-window -t '{}'", pad_win_target));
    restore_parts.push(format!("tmux select-pane -t '{}'", pad_pane_id));
    restore_parts.push(shell_log_cmd(&format!(
        "after_return_select pad_window={} pad_pane={}",
        pad_win_target, pad_pane_id
    )));

    let return_cmd = format!("{} {}", PAD_RETURN_BINDING_MARKER, restore_parts.join("; "));
    let run_shell_cmd = wrap_tmux_run_shell(&return_cmd);

    let _ = run_tmux_logged(
        "install_return_bindings.bind_f12",
        vec![
            "bind-key".to_string(),
            "-T".to_string(),
            "root".to_string(),
            "F12".to_string(),
            "run-shell".to_string(),
            run_shell_cmd.clone(),
        ],
    );
    let _ = run_tmux_logged(
        "install_return_bindings.bind_cq",
        vec![
            "bind-key".to_string(),
            "-T".to_string(),
            "root".to_string(),
            "C-q".to_string(),
            "run-shell".to_string(),
            run_shell_cmd.clone(),
        ],
    );
    let sider_cmd = wrap_tmux_run_shell(&pad_sider_toggle_command());
    for key in PAD_SIDER_TOGGLE_KEYS {
        let _ = run_tmux_logged(
            &format!("install_return_bindings.bind_sider_{}", key),
            vec![
                "bind-key".to_string(),
                "-T".to_string(),
                "root".to_string(),
                (*key).to_string(),
                "run-shell".to_string(),
                sider_cmd.clone(),
            ],
        );
    }

    log_debug!(
        "handoff trace={} stage=attach.return_cmd cmd={}",
        trace_id,
        run_shell_cmd
    );
    should_zoom
}

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
