mod context {
    use crate::app::App;
    use crate::log_debug;

    use super::super::super::tmux::writable_client_for_pane;

    pub(super) struct InstallContext {
        pub(super) trace_id: String,
        pub(super) pad_pane_id: String,
        pub(super) pad_win_target: String,
        pub(super) pad_session: String,
        pub(super) pad_client: Option<String>,
    }

    impl InstallContext {
        pub(super) fn load(
            app: &mut App,
            target_pane_id: &str,
            target_session: &str,
        ) -> Option<Self> {
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
                    return None;
                }
            };

            log_debug!(
                "handoff trace={} stage=attach.begin target_pane={} target_session={} pad_pane={}",
                trace_id,
                target_pane_id,
                target_session,
                pad_pane_id
            );

            let pad_win_target = tmux_display(&pad_pane_id, "#{session_name}:#{window_index}")?;
            if pad_win_target.is_empty() {
                log_debug!(
                    "install_return_bindings: pad_win_target empty, pad_pane_id={}",
                    pad_pane_id
                );
                return None;
            }

            let pad_session = tmux_display(&pad_pane_id, "#{session_name}")?;
            if pad_session.is_empty() {
                log_debug!(
                    "install_return_bindings: pad_session empty, pad_pane_id={}",
                    pad_pane_id
                );
                return None;
            }
            let pad_client = writable_client_for_pane(&pad_pane_id);

            Some(Self {
                trace_id,
                pad_pane_id,
                pad_win_target,
                pad_session,
                pad_client,
            })
        }
    }

    fn tmux_display(target: &str, format: &str) -> Option<String> {
        std::process::Command::new("tmux")
            .args(["display-message", "-t", target, "-p", format])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}
mod return_cmd;
mod saved {
    use crate::app::App;
    use crate::log_debug;
    use crate::tmux_bindings::{current_root_binding, PAD_SIDER_TOGGLE_KEYS};

    use super::super::super::tmux::summarize_log_text;

    pub(super) struct SavedBindings {
        pub(super) f12: Option<String>,
        pub(super) cq: Option<String>,
        pub(super) sider: Vec<(&'static str, Option<String>)>,
    }

    impl SavedBindings {
        pub(super) fn capture_into_app(app: &mut App) -> Self {
            let f12 = current_root_binding("F12");
            let cq = current_root_binding("C-q");
            let sider = PAD_SIDER_TOGGLE_KEYS
                .iter()
                .map(|key| (*key, current_root_binding(key)))
                .collect::<Vec<_>>();

            app.saved_tmux_bindings.clear();
            if let Some(line) = &f12 {
                app.saved_tmux_bindings.push(line.clone());
            }
            if let Some(line) = &cq {
                app.saved_tmux_bindings.push(line.clone());
            }
            for (_, saved_binding) in &sider {
                if let Some(line) = saved_binding {
                    app.saved_tmux_bindings.push(line.clone());
                }
            }

            log_debug!(
                "install_return_bindings: saved_bindings f12={} cq={}",
                f12.as_deref()
                    .map(summarize_log_text)
                    .unwrap_or_else(|| "-".to_string()),
                cq.as_deref()
                    .map(summarize_log_text)
                    .unwrap_or_else(|| "-".to_string())
            );

            Self { f12, cq, sider }
        }
    }
}
mod zoom {
    use crate::app::App;

    pub(super) struct ZoomDecision {
        pub(super) already_zoomed: bool,
        pub(super) pane_count: usize,
        pub(super) should_zoom: bool,
        pub(super) restore_zoom_cmd: String,
    }

    impl ZoomDecision {
        pub(super) fn for_target(app: &App, target_pane_id: &str) -> Self {
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
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                .unwrap_or_default();

            let mut parts = zoom_info.split_whitespace();
            let already_zoomed = parts.next().unwrap_or("0") == "1";
            let pane_count = parts.next().unwrap_or("1").parse().unwrap_or(1);
            let want_zoom = app.config.desired_agent_style.zoom == "auto";
            let should_zoom = want_zoom && pane_count > 1 && !already_zoomed;
            let restore_zoom_cmd = if should_zoom {
                // Do NOT zoom here — zoom happens after select-pane so user sees it instantly.
                format!("tmux resize-pane -Z -t '{}'", target_pane_id)
            } else {
                String::new()
            };

            Self {
                already_zoomed,
                pane_count,
                should_zoom,
                restore_zoom_cmd,
            }
        }
    }
}

use crate::app::App;
use crate::log_debug;
use crate::tmux_bindings::{pad_sider_toggle_command, PAD_SIDER_TOGGLE_KEYS};

use super::super::tmux::{
    apply_desired_status, run_tmux_logged, tmux_status_value, wrap_tmux_run_shell,
};
use context::InstallContext;
use return_cmd::build_return_run_shell_cmd;
use saved::SavedBindings;
use zoom::ZoomDecision;

/// Install F12/C-q/F10/C-Tab bindings for same-session attach.
/// Snapshots zoom and status bar state, modifies them for the attach,
/// and encodes restoration into the return command.
pub(in crate::event::attach) fn install_return_bindings(
    app: &mut App,
    target_pane_id: &str,
    target_session: &str,
) -> bool {
    let Some(ctx) = InstallContext::load(app, target_pane_id, target_session) else {
        return false;
    };

    let zoom = ZoomDecision::for_target(app, target_pane_id);
    let saved_bindings = SavedBindings::capture_into_app(app);
    let (status_val, desired_status, status_restore_value, restore_status_cmd) =
        apply_attach_status(app, target_session);

    log_debug!(
        "install_return_bindings: target={} target_session={} panes={} zoomed={} should_zoom={} status={} desired_status={} status_restore={} pad_session={} pad_win={}",
        target_pane_id,
        target_session,
        zoom.pane_count,
        zoom.already_zoomed,
        zoom.should_zoom,
        status_val,
        desired_status,
        status_restore_value.as_deref().unwrap_or("-"),
        ctx.pad_session,
        ctx.pad_win_target
    );

    let run_shell_cmd = build_return_run_shell_cmd(
        &ctx,
        target_pane_id,
        target_session,
        &saved_bindings,
        &zoom.restore_zoom_cmd,
        &restore_status_cmd,
    );
    install_return_keys(&run_shell_cmd);
    install_sider_toggle_keys();

    log_debug!(
        "handoff trace={} stage=attach.return_cmd cmd={}",
        ctx.trace_id,
        run_shell_cmd
    );
    zoom.should_zoom
}

fn apply_attach_status(
    app: &mut App,
    target_session: &str,
) -> (String, String, Option<String>, String) {
    let status_val = tmux_status_value(Some(target_session));
    let desired_status = app.config.desired_agent_style.status.clone();
    let status_restore_value = apply_desired_status(&desired_status, &status_val, target_session);

    app.saved_tmux_status = status_restore_value.clone();
    app.saved_tmux_status_target = status_restore_value
        .as_ref()
        .map(|_| target_session.to_string());

    let restore_status_cmd = status_restore_value
        .as_ref()
        .map(|status| format!("tmux set -t '{}' status '{}'", target_session, status))
        .unwrap_or_default();

    (
        status_val,
        desired_status,
        status_restore_value,
        restore_status_cmd,
    )
}

fn install_return_keys(run_shell_cmd: &str) {
    bind_root_key(
        "install_return_bindings.bind_f12",
        "F12",
        run_shell_cmd.to_string(),
    );
    bind_root_key(
        "install_return_bindings.bind_cq",
        "C-q",
        run_shell_cmd.to_string(),
    );
}

fn install_sider_toggle_keys() {
    let sider_cmd = wrap_tmux_run_shell(&pad_sider_toggle_command());
    for key in PAD_SIDER_TOGGLE_KEYS {
        bind_root_key(
            &format!("install_return_bindings.bind_sider_{}", key),
            key,
            sider_cmd.clone(),
        );
    }
}

fn bind_root_key(context: &str, key: &str, run_shell_cmd: String) {
    let _ = run_tmux_logged(
        context,
        vec![
            "bind-key".to_string(),
            "-T".to_string(),
            "root".to_string(),
            key.to_string(),
            "run-shell".to_string(),
            run_shell_cmd,
        ],
    );
}
