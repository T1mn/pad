use std::collections::HashSet;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use super::client::spawn_control_client;
use super::parser::{parse_control_event, ParsedControlEvent};
use super::TmuxEvent;

/// Start the tmux control mode pipe listener.
/// Returns a receiver that emits TmuxEvent when tmux state changes.
pub(crate) fn start_control_pipe() -> mpsc::Receiver<TmuxEvent> {
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
    let session_name = current_tmux_session_name().ok_or("Cannot determine tmux session name")?;
    let mut child = spawn_control_client(&session_name).await?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout).lines();
    let mut seen_ignored_types = HashSet::new();

    log_debug!("tmux_pipe: connected to session '{}'", session_name);

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

    let _ = child.kill().await;
    Ok(())
}

fn current_tmux_session_name() -> Option<String> {
    std::env::var("TMUX_PANE")
        .ok()
        .and_then(|_| {
            std::process::Command::new("tmux")
                .args(["display-message", "-p", "#{session_name}"])
                .output()
                .ok()
        })
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|session| !session.is_empty())
}
