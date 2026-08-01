use super::stream::handle_direct_hook_stream;
use crate::log_debug;
use crate::socket_api::peer::authorize_peer;
use crate::socket_api::socket_file::{bind_private_listener, socket_is_live};
use std::io;
use tokio::net::UnixListener;

pub(in crate::chat::providers::telegram) fn daemon_socket_is_active() -> bool {
    let path = crate::paths::telegram_hook_socket_path();
    socket_is_live(&path)
}

pub(in crate::chat::providers::telegram) fn start_direct_hook_listener() -> io::Result<()> {
    let socket_path = crate::paths::telegram_hook_socket_path();
    let listener = UnixListener::from_std(bind_private_listener(&socket_path)?)?;

    tokio::spawn(async move {
        log_debug!(
            "telegram: direct hook listener on {}",
            socket_path.display()
        );

        accept_direct_hook_streams(listener).await;
    });
    Ok(())
}

async fn accept_direct_hook_streams(listener: UnixListener) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                if let Err(err) = authorize_peer(&stream) {
                    log_debug!("telegram: rejected direct hook connection: {}", err);
                    drop(stream);
                    continue;
                }
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

#[cfg(test)]
#[path = "listener_tests.rs"]
mod tests;
