use crate::log_debug;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookTmuxInfo {
    pub pane_id: Option<String>,
    pub session_name: Option<String>,
    pub window_index: Option<String>,
    pub pane_index: Option<String>,
    pub pane_current_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event: String,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub cwd: Option<String>,
    pub prompt: Option<String>,
    pub last_assistant_message: Option<String>,
    pub timestamp: Option<String>,
    pub tmux: HookTmuxInfo,
}

pub fn start_hook_listener() -> mpsc::Receiver<HookEvent> {
    let socket_path = crate::paths::hook_socket_path();
    if let Some(parent) = socket_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::remove_file(&socket_path);
    let (tx, rx) = mpsc::channel::<HookEvent>(32);

    tokio::spawn(async move {
        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => l,
            Err(e) => {
                log_debug!("hook_listener: bind failed: {}", e);
                return;
            }
        };
        log_debug!("hook_listener: listening on {}", display_path(&socket_path));

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_stream(stream, tx).await {
                            log_debug!("hook_listener: stream error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log_debug!("hook_listener: accept error: {}", e);
                    break;
                }
            }
        }
    });

    rx
}

fn display_path(path: &PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

async fn handle_stream(
    stream: UnixStream,
    tx: mpsc::Sender<HookEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let event: HookEvent = serde_json::from_str(&line)?;
        log_debug!(
            "hook_listener: event={} pane={:?} cwd={:?}",
            event.event,
            event.tmux.pane_id,
            event.cwd
        );
        append_hook_event_journal(&event);
        let _ = tx.send(event).await;
    }

    Ok(())
}

fn append_hook_event_journal(event: &HookEvent) {
    let path = crate::paths::hook_events_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut file) => {
            if let Ok(line) = serde_json::to_string(event) {
                let _ = writeln!(file, "{}", line);
            }
        }
        Err(err) => {
            log_debug!("hook_listener: failed to append hook journal: {}", err);
        }
    }
}
