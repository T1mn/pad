use crate::log_debug;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::mpsc;
use std::time::Duration;

/// Events emitted by the tmux control pipe
#[derive(Debug)]
pub enum TmuxEvent {
    /// A window was added/removed/changed
    WindowChanged,
    /// A pane mode changed (could indicate state change)
    PaneModeChanged,
    /// Session changed
    SessionChanged,
    /// Output detected on a pane
    OutputDetected,
    /// Pipe disconnected
    Disconnected,
}

/// Start the tmux control mode pipe listener.
/// Returns a receiver that emits TmuxEvent when tmux state changes.
pub fn start_control_pipe() -> mpsc::Receiver<TmuxEvent> {
    let (tx, rx) = mpsc::channel::<TmuxEvent>(32);

    tokio::spawn(async move {
        let mut backoff_ms: u64 = 2000;
        let max_backoff_ms: u64 = 30000;

        loop {
            log_debug!("tmux_pipe: connecting to control mode...");
            match run_pipe(&tx).await {
                Ok(()) => {
                    log_debug!("tmux_pipe: pipe closed normally");
                }
                Err(e) => {
                    log_debug!("tmux_pipe: error: {}", e);
                }
            }

            // Notify disconnect
            let _ = tx.send(TmuxEvent::Disconnected).await;

            if tx.is_closed() {
                log_debug!("tmux_pipe: receiver dropped, exiting");
                break;
            }

            log_debug!("tmux_pipe: reconnecting in {}ms", backoff_ms);
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
        }
    });

    rx
}

async fn run_pipe(tx: &mpsc::Sender<TmuxEvent>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Start tmux in control mode, attached to current session
    let session_name = std::env::var("TMUX_PANE").ok().and_then(|_| {
        std::process::Command::new("tmux")
            .args(["display-message", "-p", "#{session_name}"])
            .output()
            .ok()
    }).and_then(|o| if o.status.success() {
        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        None
    }).unwrap_or_default();

    if session_name.is_empty() {
        return Err("Cannot determine tmux session name".into());
    }

    let mut child = TokioCommand::new("tmux")
        .args(["-C", "attach-session", "-t", &session_name, "-r"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout).lines();

    log_debug!("tmux_pipe: connected to session '{}'", session_name);

    // Read tmux control mode events line by line
    while let Some(line) = reader.next_line().await? {
        if tx.is_closed() {
            break;
        }

        // tmux control mode events start with %
        let event = if line.starts_with("%window-add")
            || line.starts_with("%window-close")
            || line.starts_with("%window-renamed")
        {
            Some(TmuxEvent::WindowChanged)
        } else if line.starts_with("%session-changed")
            || line.starts_with("%session-renamed")
            || line.starts_with("%sessions-changed")
        {
            Some(TmuxEvent::SessionChanged)
        } else if line.starts_with("%pane-mode-changed") {
            Some(TmuxEvent::PaneModeChanged)
        } else if line.starts_with("%output") {
            Some(TmuxEvent::OutputDetected)
        } else {
            None
        };

        if let Some(ev) = event {
            log_debug!("tmux_pipe: event {:?}", ev);
            if tx.send(ev).await.is_err() {
                break;
            }
        }
    }

    // Clean up child process
    let _ = child.kill().await;
    Ok(())
}
