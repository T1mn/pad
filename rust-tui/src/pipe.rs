use std::collections::HashSet;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::mpsc;

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

#[derive(Debug)]
enum ParsedControlEvent<'a> {
    Emit {
        raw_type: &'a str,
        event: TmuxEvent,
    },
    Ignore {
        raw_type: &'a str,
        reason: &'static str,
    },
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

async fn run_pipe(
    tx: &mpsc::Sender<TmuxEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Start tmux in control mode, attached to current session
    let session_name = std::env::var("TMUX_PANE")
        .ok()
        .and_then(|_| {
            std::process::Command::new("tmux")
                .args(["display-message", "-p", "#{session_name}"])
                .output()
                .ok()
        })
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default();

    if session_name.is_empty() {
        return Err("Cannot determine tmux session name".into());
    }

    let mut child = TokioCommand::new("tmux")
        // no-output disables noisy %output notifications; pad only needs
        // structural/session/mode events from control mode.
        .args([
            "-C",
            "attach-session",
            "-t",
            &session_name,
            "-f",
            "read-only,ignore-size,no-output",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout).lines();
    let mut seen_ignored_types = HashSet::new();

    log_debug!("tmux_pipe: connected to session '{}'", session_name);

    // Read tmux control mode events line by line
    while let Some(line) = reader.next_line().await? {
        if tx.is_closed() {
            break;
        }

        if let Some(parsed) = parse_control_event(&line) {
            match parsed {
                ParsedControlEvent::Emit { raw_type, event } => {
                    if !matches!(event, TmuxEvent::OutputDetected) {
                        log_debug!("tmux_pipe: event type={} mapped={:?}", raw_type, event);
                    }
                    if tx.send(event).await.is_err() {
                        break;
                    }
                }
                ParsedControlEvent::Ignore { raw_type, reason } => {
                    if seen_ignored_types.insert(raw_type.to_string()) {
                        log_debug!("tmux_pipe: ignoring type={} reason={}", raw_type, reason);
                    }
                }
            }
        }
    }

    // Clean up child process
    let _ = child.kill().await;
    Ok(())
}

fn parse_control_event(line: &str) -> Option<ParsedControlEvent<'_>> {
    let raw_type = line.split_whitespace().next()?;
    if !raw_type.starts_with('%') {
        return None;
    }

    match raw_type {
        "%window-add"
        | "%window-close"
        | "%window-renamed"
        | "%window-pane-changed"
        | "%layout-change" => Some(ParsedControlEvent::Emit {
            raw_type,
            event: TmuxEvent::WindowChanged,
        }),
        "%session-changed"
        | "%session-renamed"
        | "%sessions-changed"
        | "%session-window-changed" => Some(ParsedControlEvent::Emit {
            raw_type,
            event: TmuxEvent::SessionChanged,
        }),
        "%pane-mode-changed" => Some(ParsedControlEvent::Emit {
            raw_type,
            event: TmuxEvent::PaneModeChanged,
        }),
        "%output" | "%extended-output" => Some(ParsedControlEvent::Emit {
            raw_type,
            event: TmuxEvent::OutputDetected,
        }),
        "%begin" | "%end" | "%error" => Some(ParsedControlEvent::Ignore {
            raw_type,
            reason: "protocol frame",
        }),
        "%message"
        | "%client-detached"
        | "%client-session-changed"
        | "%config-error"
        | "%continue"
        | "%pause"
        | "%paste-buffer-changed"
        | "%paste-buffer-deleted"
        | "%subscription-changed"
        | "%unlinked-window-add"
        | "%unlinked-window-close"
        | "%unlinked-window-renamed"
        | "%exit" => Some(ParsedControlEvent::Ignore {
            raw_type,
            reason: "not relevant to panel scan",
        }),
        _ => Some(ParsedControlEvent::Ignore {
            raw_type,
            reason: "unrecognized control notification",
        }),
    }
}
