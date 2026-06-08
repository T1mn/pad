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

use super::tmux::{apply_desired_status, tmux_status_value};

pub(super) fn attach_via_pty(
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
