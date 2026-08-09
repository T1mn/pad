use super::model::{ApiRequest, ApiResponse};
use super::peer::authorize_peer;
use super::socket_file::bind_private_listener;
use std::io;
use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

const UI_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const BROWSER_OPEN_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_COMMAND_OUTPUT_BYTES_PER_STREAM: usize = 1024 * 1024;
const CALL_PENDING: u8 = 0;
const CALL_EXECUTING: u8 = 1;
const CALL_CANCELLED: u8 = 2;

pub struct ApiCall {
    pub request: ApiRequest,
    pub response: oneshot::Sender<ApiResponse>,
    pub(super) deadline: Instant,
    pub(super) state: Arc<AtomicU8>,
}

impl ApiCall {
    pub fn try_begin(&self) -> bool {
        if self.response.is_closed() || Instant::now() >= self.deadline {
            let _ = self.state.compare_exchange(
                CALL_PENDING,
                CALL_CANCELLED,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            return false;
        }
        self.state
            .compare_exchange(
                CALL_PENDING,
                CALL_EXECUTING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }
}

pub type ApiReceiver = mpsc::Receiver<ApiCall>;

/// bind 在同步部分完成，失败会真正返回给调用方；只有 accept 循环进 tokio::spawn。
pub fn start_api_listener() -> io::Result<ApiReceiver> {
    let socket_path = crate::paths::api_socket_path();
    let listener = bind_private_listener(&socket_path)?;
    let listener = UnixListener::from_std(listener)?;
    log_debug!("socket_api: listening on {}", display_path(&socket_path));
    let (sender, receiver) = mpsc::channel(32);
    tokio::spawn(accept_loop(listener, sender));
    Ok(receiver)
}

async fn accept_loop(listener: UnixListener, sender: mpsc::Sender<ApiCall>) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                if let Err(err) = authorize_peer(&stream) {
                    log_debug!("socket_api: rejected connection: {}", err);
                    drop(stream);
                    continue;
                }
                let sender = sender.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle_stream(stream, sender).await {
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

async fn handle_stream(stream: UnixStream, sender: mpsc::Sender<ApiCall>) -> io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<ApiRequest>(&line) {
            Ok(request) => dispatch_request(request, &sender).await,
            Err(err) => ApiResponse::err(format!("invalid request json: {err}")),
        };
        let encoded = serde_json::to_string(&response)?;
        writer.write_all(encoded.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }
    Ok(())
}

async fn dispatch_request(request: ApiRequest, sender: &mpsc::Sender<ApiCall>) -> ApiResponse {
    if requires_ui_state(&request.action) {
        dispatch_ui_request(request, sender).await
    } else {
        dispatch_background_request(request).await
    }
}

pub(super) fn requires_ui_state(action: &str) -> bool {
    matches!(
        action,
        "status" | "prompt" | "capture" | "escape" | "approval"
    )
}

async fn dispatch_ui_request(request: ApiRequest, sender: &mpsc::Sender<ApiCall>) -> ApiResponse {
    let deadline = Instant::now() + UI_REQUEST_TIMEOUT;
    let state = Arc::new(AtomicU8::new(CALL_PENDING));
    let (response, receiver) = oneshot::channel();
    let call = ApiCall {
        request,
        response,
        deadline,
        state: Arc::clone(&state),
    };
    if tokio::time::timeout(UI_REQUEST_TIMEOUT, sender.send(call))
        .await
        .map_or(true, |result| result.is_err())
    {
        let _ = state.compare_exchange(
            CALL_PENDING,
            CALL_CANCELLED,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        return ApiResponse::err("PAD native runtime is shutting down or busy");
    }

    let mut receiver = receiver;
    let remaining = deadline.saturating_duration_since(Instant::now());
    match tokio::time::timeout(remaining, &mut receiver).await {
        Ok(Ok(response)) => response,
        Ok(Err(_)) => ApiResponse::err("PAD native runtime dropped the request"),
        Err(_) => {
            if state
                .compare_exchange(
                    CALL_PENDING,
                    CALL_CANCELLED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                ApiResponse::err("PAD native runtime request timed out")
            } else if state.load(Ordering::Acquire) == CALL_EXECUTING {
                receiver.await.unwrap_or_else(|_| {
                    ApiResponse::err("PAD native runtime dropped the executing request")
                })
            } else {
                ApiResponse::err("PAD native runtime request was cancelled")
            }
        }
    }
}

async fn dispatch_background_request(request: ApiRequest) -> ApiResponse {
    match request.action.as_str() {
        "browser_open" => browser_open_response(request).await,
        "remote_exec" => remote_exec_response(request).await,
        _ => match tokio::task::spawn_blocking(move || super::handler::handle_request(request))
            .await
        {
            Ok(response) => response,
            Err(error) => ApiResponse::err(format!("PAD background request failed: {error}")),
        },
    }
}

async fn browser_open_response(request: ApiRequest) -> ApiResponse {
    let Some(url) = request.url.as_deref() else {
        return ApiResponse::err("missing url");
    };
    let command = match crate::browser_remote::browser_open_command(url) {
        Ok(command) => command,
        Err(error) => return ApiResponse::err(format!("browser command failed: {error}")),
    };
    if request.dry_run {
        return ApiResponse::ok(
            "dry_run",
            Some(serde_json::json!({ "program": command.program, "args": command.args })),
        );
    }
    match command_output(command.program, command.args, BROWSER_OPEN_TIMEOUT).await {
        Ok(output) if output.status.success() => ApiResponse::ok("opened", None),
        Ok(output) => ApiResponse::err(format!(
            "browser open failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(error) => ApiResponse::err(format!("browser open failed: {error}")),
    }
}

async fn remote_exec_response(request: ApiRequest) -> ApiResponse {
    let ssh = match super::handler::remote_exec_command(&request) {
        Ok(ssh) => ssh,
        Err(response) => return response,
    };
    if request.dry_run {
        return ApiResponse::ok("dry_run", Some(serde_json::json!({ "command": ssh })));
    }
    let program = ssh[0].clone();
    let args = ssh[1..].to_vec();
    match command_output_bounded(program, args).await {
        Ok(output) if output.status.success() => ApiResponse::ok(
            "ok",
            Some(serde_json::json!({
                "stdout": String::from_utf8_lossy(&output.stdout).into_owned()
            })),
        ),
        Ok(output) => ApiResponse::err(format!(
            "remote exec failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(error) => ApiResponse::err(format!("remote exec failed: {error}")),
    }
}

async fn command_output(
    program: String,
    args: Vec<String>,
    timeout: Duration,
) -> Result<CapturedOutput, String> {
    match tokio::time::timeout(timeout, command_output_bounded(program, args)).await {
        Ok(output) => output,
        Err(_) => Err(format!(
            "command timed out after {} seconds",
            timeout.as_secs()
        )),
    }
}

#[derive(Debug)]
pub(super) struct CapturedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

pub(super) async fn command_output_bounded(
    program: String,
    args: Vec<String>,
) -> Result<CapturedOutput, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "command stdout pipe is unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "command stderr pipe is unavailable".to_string())?;
    let readers = async move {
        tokio::try_join!(
            read_limited(stdout, "stdout"),
            read_limited(stderr, "stderr")
        )
    };
    tokio::pin!(readers);

    enum First {
        Readers(Result<(Vec<u8>, Vec<u8>), String>),
        Status(io::Result<ExitStatus>),
    }
    let first = tokio::select! {
        output = &mut readers => First::Readers(output),
        status = child.wait() => First::Status(status),
    };
    match first {
        First::Readers(Ok((stdout, stderr))) => {
            let status = child.wait().await.map_err(|error| error.to_string())?;
            Ok(CapturedOutput {
                status,
                stdout,
                stderr,
            })
        }
        First::Readers(Err(error)) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(error)
        }
        First::Status(status) => {
            let status = status.map_err(|error| error.to_string())?;
            let (stdout, stderr) = readers.await?;
            Ok(CapturedOutput {
                status,
                stdout,
                stderr,
            })
        }
    }
}

async fn read_limited(mut reader: impl AsyncRead + Unpin, stream: &str) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let read = reader
            .read(&mut chunk)
            .await
            .map_err(|error| error.to_string())?;
        if read == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(read) > MAX_COMMAND_OUTPUT_BYTES_PER_STREAM {
            return Err(format!(
                "{stream} exceeded the {} byte limit; command was stopped and its remote outcome may be unknown",
                MAX_COMMAND_OUTPUT_BYTES_PER_STREAM
            ));
        }
        output.extend_from_slice(&chunk[..read]);
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
