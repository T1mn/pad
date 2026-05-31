use super::handler::handle_request;
use super::model::{ApiRequest, ApiResponse};
use std::io;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

pub fn api_socket_is_active() -> bool {
    let path = crate::paths::api_socket_path();
    path.exists() && StdUnixStream::connect(path).is_ok()
}

pub fn start_api_listener() -> io::Result<()> {
    let socket_path = crate::paths::api_socket_path();
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        if api_socket_is_active() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("pad socket API already active at {}", socket_path.display()),
            ));
        }
        match std::fs::remove_file(&socket_path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    tokio::spawn(async move {
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                log_debug!("socket_api: bind failed: {}", err);
                return;
            }
        };
        log_debug!("socket_api: listening on {}", display_path(&socket_path));
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        if let Err(err) = handle_stream(stream).await {
                            log_debug!("socket_api: stream error: {}", err);
                        }
                    });
                }
                Err(err) => {
                    log_debug!("socket_api: accept error: {}", err);
                    break;
                }
            }
        }
    });
    Ok(())
}

async fn handle_stream(stream: UnixStream) -> io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<ApiRequest>(&line) {
            Ok(request) => handle_request(request),
            Err(err) => ApiResponse::err(format!("invalid request json: {err}")),
        };
        let encoded = serde_json::to_string(&response)?;
        writer.write_all(encoded.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }
    Ok(())
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
