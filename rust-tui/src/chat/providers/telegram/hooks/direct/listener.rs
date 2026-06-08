use super::stream::handle_direct_hook_stream;
use crate::log_debug;
use std::io;
use std::os::unix::net::UnixStream as StdUnixStream;
use tokio::net::UnixListener;

pub(in crate::chat::providers::telegram) fn daemon_socket_is_active() -> bool {
    let path = crate::paths::telegram_hook_socket_path();
    path.exists() && StdUnixStream::connect(path).is_ok()
}

pub(in crate::chat::providers::telegram) fn start_direct_hook_listener() -> io::Result<()> {
    let socket_path = crate::paths::telegram_hook_socket_path();
    prepare_hook_socket(&socket_path)?;

    tokio::spawn(async move {
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                log_debug!("telegram: direct hook bind failed: {}", err);
                return;
            }
        };
        log_debug!(
            "telegram: direct hook listener on {}",
            socket_path.display()
        );

        accept_direct_hook_streams(listener).await;
    });
    Ok(())
}

fn prepare_hook_socket(socket_path: &std::path::Path) -> io::Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !socket_path.exists() {
        return Ok(());
    }

    if daemon_socket_is_active() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "telegram daemon socket already active at {}",
                socket_path.display()
            ),
        ));
    }

    match std::fs::remove_file(socket_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

async fn accept_direct_hook_streams(listener: UnixListener) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(async move {
                    if let Err(err) = handle_direct_hook_stream(stream).await {
                        log_debug!("telegram: direct hook stream error: {}", err);
                    }
                });
            }
            Err(err) => {
                log_debug!("telegram: direct hook accept error: {}", err);
                break;
            }
        }
    }
}
