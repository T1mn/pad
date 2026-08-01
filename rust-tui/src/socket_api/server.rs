use super::handler::handle_request;
use super::model::{ApiRequest, ApiResponse};
use super::peer::authorize_peer;
use super::socket_file::bind_private_listener;
use std::io;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

/// bind 在同步部分完成，失败会真正返回给调用方；只有 accept 循环进 tokio::spawn。
pub fn start_api_listener() -> io::Result<()> {
    let socket_path = crate::paths::api_socket_path();
    let listener = bind_private_listener(&socket_path)?;
    let listener = UnixListener::from_std(listener)?;
    log_debug!("socket_api: listening on {}", display_path(&socket_path));
    tokio::spawn(accept_loop(listener));
    Ok(())
}

async fn accept_loop(listener: UnixListener) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                if let Err(err) = authorize_peer(&stream) {
                    log_debug!("socket_api: rejected connection: {}", err);
                    drop(stream);
                    continue;
                }
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

#[cfg(test)]
#[path = "server_tests.rs"]
mod server_tests;
