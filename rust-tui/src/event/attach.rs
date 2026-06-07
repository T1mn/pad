mod bindings;
mod tmux;

pub(super) use bindings::restore_tmux_bindings;
pub(super) use tmux::current_tmux_pane_id;

use crate::app::App;
use crate::log_debug;
use crate::model::AgentPanel;
use crate::pty::attach_to_pane_pty;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::time::Duration;

use bindings::install_return_bindings;
use tmux::{
    apply_desired_status, current_tmux_session, current_tmux_window_target, run_tmux_success,
    tmux_status_value, tmux_target_snapshot,
};

pub(super) fn handle_attach(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    if let Some(panel) = app.selected_panel() {
        let panel = panel.clone();
        log_debug!(
            "attach: pane_id={} agent={} session={} window={}",
            panel.pane_id,
            panel.agent_type,
            panel.session,
            panel.window_index
        );

        // Clean up any stale temporary bindings before installing a fresh handoff.
        if !app.saved_tmux_bindings.is_empty() || app.same_session_attached {
            restore_tmux_bindings(app);
            app.same_session_attached = false;
        }

        // Detect if target pane is in the same tmux session
        let current_session = std::env::var("TMUX_PANE")
            .ok()
            .and_then(|_| current_tmux_session());

        log_debug!(
            "attach: current_session={} target_session={} current_window={} current_pane={}",
            current_session.as_deref().unwrap_or("-"),
            panel.session,
            current_tmux_window_target().as_deref().unwrap_or("-"),
            current_tmux_pane_id().as_deref().unwrap_or("-")
        );

        if let Some(current_session) = current_session.as_deref() {
            let cross_session = current_session != panel.session;
            if handoff_to_tmux_client(app, &panel, current_session, cross_session) {
                return Ok(());
            }

            if !cross_session {
                log_debug!("attach.same_session: handoff failed, leaving pad in place");
                app.dirty = true;
                return Ok(());
            }

            log_debug!(
                "attach.cross_session: client handoff failed current_session={} target_session={}, falling back to PTY",
                current_session,
                panel.session
            );
        }

        attach_via_pty(terminal, app, &panel)?;
    } else {
        log_debug!("attach: no panel selected");
    }
    Ok(())
}

fn handoff_to_tmux_client(
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

    log_debug!(
        "{}: start target_window={} target_pane={} current_session={} current_window={} current_pane={} target_snapshot={}",
        prefix,
        target_window,
        panel.pane_id,
        current_session,
        current_tmux_window_target().as_deref().unwrap_or("-"),
        current_tmux_pane_id().as_deref().unwrap_or("-"),
        tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
    );

    let should_zoom = install_return_bindings(app, &panel.pane_id, &panel.session);

    if cross_session
        && !run_tmux_success(
            "attach.cross_session.switch_client",
            vec![
                "switch-client".to_string(),
                "-t".to_string(),
                panel.session.clone(),
            ],
        )
    {
        log_debug!(
            "attach.cross_session: switch-client failed target_session={}",
            panel.session
        );
        restore_tmux_bindings(app);
        return false;
    }

    if cross_session {
        log_debug!(
            "attach.cross_session: after switch-client current_window={} current_pane={} target_snapshot={}",
            current_tmux_window_target().as_deref().unwrap_or("-"),
            current_tmux_pane_id().as_deref().unwrap_or("-"),
            tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
        );
    }

    if !run_tmux_success(
        &format!("{}.select_window", prefix),
        vec![
            "select-window".to_string(),
            "-t".to_string(),
            target_window.clone(),
        ],
    ) {
        log_debug!(
            "{}: select-window failed target_window={}",
            prefix,
            target_window
        );
        restore_tmux_bindings(app);
        return false;
    }

    log_debug!(
        "{}: after select-window current_window={} current_pane={} target_snapshot={}",
        prefix,
        current_tmux_window_target().as_deref().unwrap_or("-"),
        current_tmux_pane_id().as_deref().unwrap_or("-"),
        tmux_target_snapshot(&panel.pane_id)
            .as_deref()
            .unwrap_or("-")
    );

    if !run_tmux_success(
        &format!("{}.select_pane", prefix),
        vec![
            "select-pane".to_string(),
            "-t".to_string(),
            panel.pane_id.clone(),
        ],
    ) {
        log_debug!(
            "{}: select-pane failed target_pane={}",
            prefix,
            panel.pane_id
        );
        restore_tmux_bindings(app);
        return false;
    }

    log_debug!(
        "{}: after select-pane current_window={} current_pane={} target_snapshot={}",
        prefix,
        current_tmux_window_target().as_deref().unwrap_or("-"),
        current_tmux_pane_id().as_deref().unwrap_or("-"),
        tmux_target_snapshot(&panel.pane_id)
            .as_deref()
            .unwrap_or("-")
    );

    if should_zoom
        && !run_tmux_success(
            &format!("{}.resize_zoom", prefix),
            vec![
                "resize-pane".to_string(),
                "-Z".to_string(),
                "-t".to_string(),
                panel.pane_id.clone(),
            ],
        )
    {
        log_debug!(
            "{}: resize-pane failed target_pane={}",
            prefix,
            panel.pane_id
        );
        restore_tmux_bindings(app);
        return false;
    }

    if should_zoom {
        log_debug!(
            "{}: after resize-pane current_window={} current_pane={} target_snapshot={}",
            prefix,
            current_tmux_window_target().as_deref().unwrap_or("-"),
            current_tmux_pane_id().as_deref().unwrap_or("-"),
            tmux_target_snapshot(&panel.pane_id)
                .as_deref()
                .unwrap_or("-")
        );
    }

    app.same_session_attached = true;
    log_debug!(
        "{}: handoff complete target_window={} target_pane={} should_zoom={} target_snapshot={}",
        prefix,
        target_window,
        panel.pane_id,
        should_zoom,
        tmux_target_snapshot(&panel.pane_id)
            .as_deref()
            .unwrap_or("-")
    );
    app.dirty = true;
    true
}

fn attach_via_pty(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    panel: &AgentPanel,
) -> io::Result<()> {
    let status_before = tmux_status_value(Some(&panel.session));
    let desired_status = app.config.desired_agent_style.status.as_str();
    let status_restore_value = apply_desired_status(desired_status, &status_before, &panel.session);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    print!("\x1b[2J\x1b[H");
    println!(
        "Attaching to {} @ {} (window {})",
        panel.agent_type, panel.pane_id, panel.window_index
    );
    println!("Press F12, Ctrl+Q, or Ctrl+B then d to return to pad\n");
    io::stdout().flush()?;

    std::thread::sleep(Duration::from_millis(100));

    match attach_to_pane_pty(panel) {
        Ok(()) => {
            log_debug!("attach: detached normally from pane_id={}", panel.pane_id);
        }
        Err(e) => {
            log_debug!("attach: ERROR pane_id={} err={}", panel.pane_id, e);
            println!("Attach error: {}", e);
            println!("Press any key to continue...");
            io::stdout().flush()?;
            let _ = crossterm::event::read();
        }
    }

    print!("\x1b[2J\x1b[H");
    io::stdout().flush()?;

    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;

    terminal.clear()?;

    if let Some(status) = status_restore_value.as_deref() {
        let _ = std::process::Command::new("tmux")
            .args(["set", "-t", &panel.session, "status", status])
            .output();
    }

    app.refresh_after_attach = true;
    app.dirty = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::bindings::{restore_binding_cmd, PAD_SIDER_TOGGLE_KEYS};

    #[test]
    fn sider_toggle_keys_include_ctrl_tab() {
        assert!(PAD_SIDER_TOGGLE_KEYS.contains(&"F10"));
        assert!(PAD_SIDER_TOGGLE_KEYS.contains(&"C-Tab"));
    }

    #[test]
    fn restore_binding_cmd_can_unbind_ctrl_tab() {
        assert_eq!(
            restore_binding_cmd(None, "C-Tab"),
            "tmux unbind-key -T root C-Tab"
        );
    }
}
