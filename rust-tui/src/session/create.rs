mod logging;
mod status;

use crate::app::App;
use std::error::Error;

use super::launch::{launch_agent_after_attach, should_launch_after_attach};
use super::pad_context::resolve_pad_context;
use super::return_bindings::{install_return_bindings, save_current_return_bindings};
use super::target::{
    create_tmux_target, parse_target_info, select_target_window, switch_client_to_target,
};
use logging::log_client_context;
use status::apply_agent_status_style;

/// Create a new tmux session in the given path with an agent command.
/// After creation, switches the tmux client to the new session and installs
/// F12/Ctrl+Q/F10/Ctrl+Tab bindings so the user can return to the pad session and
/// toggle the left helper pane inside Codex.
pub fn create_session_with_agent(
    app: &mut App,
    path: &str,
    agent_cmd: &str,
) -> Result<(), Box<dyn Error>> {
    let trace_id = crate::app::new_handoff_trace("create");
    app.same_session_trace_id = Some(trace_id.clone());
    let session_name = path.replace(['/', '.'], "_").replace('~', "home");
    let launch_after_attach = should_launch_after_attach(agent_cmd);

    log_debug!(
        "handoff trace={} stage=create.begin path={} cmd={} session_name={}",
        trace_id,
        path,
        agent_cmd,
        session_name
    );

    let target_output = create_tmux_target(path, agent_cmd, &session_name, launch_after_attach)?;
    if !target_output.status.success() {
        return Err(format!(
            "tmux create failed: {}",
            String::from_utf8_lossy(&target_output.stderr).trim()
        )
        .into());
    }

    let target = parse_target_info(&target_output, &session_name);
    log_debug!(
        "handoff trace={} stage=create.target_resolved target_window={} target_pane={}",
        trace_id,
        target.window,
        target.pane.as_deref().unwrap_or("-")
    );

    let pad = resolve_pad_context();
    log_debug!(
        "handoff trace={} stage=create.pad_context pad_pane={:?} pad_win={:?} pad_session={:?}",
        trace_id,
        pad.pane,
        pad.window,
        pad.session
    );
    log_client_context(&trace_id);

    save_current_return_bindings(app);
    let status_restore_value = apply_agent_status_style(app, &session_name, pad.session.as_deref());

    if let (Some(pane), Some(window), Some(session)) = (
        pad.pane.as_deref(),
        pad.window.as_deref(),
        pad.session.as_deref(),
    ) {
        install_return_bindings(
            app,
            &trace_id,
            &session_name,
            &target.window,
            pane,
            window,
            session,
            status_restore_value.as_deref(),
        );
    } else {
        log_debug!(
            "handoff trace={} stage=create.skip_return_binding reason=tmux_pane_missing",
            trace_id
        );
    }

    select_target_window(&target);
    switch_client_to_target(&trace_id, &session_name);

    if launch_after_attach {
        if let Some(target_pane) = target.pane.as_deref() {
            launch_agent_after_attach(target_pane, agent_cmd);
        }
    }

    Ok(())
}
