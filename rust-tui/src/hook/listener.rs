use super::journal::append_hook_event_journal;
use super::HookEvent;
use crate::log_debug;
use std::io;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

pub fn hook_socket_is_active() -> bool {
    let path = crate::paths::hook_socket_path();
    path.exists() && StdUnixStream::connect(path).is_ok()
}

pub fn start_hook_listener() -> io::Result<mpsc::Receiver<HookEvent>> {
    let socket_path = crate::paths::hook_socket_path();
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        if hook_socket_is_active() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "pad hook socket already active at {}",
                    socket_path.display()
                ),
            ));
        }
        match std::fs::remove_file(&socket_path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
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

    Ok(rx)
}

fn display_path(path: &Path) -> String {
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
