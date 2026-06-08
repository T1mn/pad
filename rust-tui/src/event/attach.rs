mod bindings;
mod client_handoff;
mod pty_attach;
mod tmux;

pub(super) use bindings::restore_tmux_bindings;
pub(super) use tmux::current_tmux_pane_id;

use crate::app::App;
use crate::log_debug;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use client_handoff::handoff_to_tmux_client;
use pty_attach::attach_via_pty;
use tmux::{current_tmux_session, current_tmux_window_target};

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

#[cfg(test)]
#[path = "attach_tests.rs"]
mod tests;
