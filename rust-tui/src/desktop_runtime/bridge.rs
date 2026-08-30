//! Line-oriented host bridge for the macOS Desktop shell.
//!
//! The bridge is intentionally transport-thin: Swift/WKWebView sends one
//! JSON request per stdin line and receives one JSON response per stdout
//! line.  All durable state and Pi process ownership stays in
//! `DesktopRuntime`; the renderer never receives a SQLite handle or child
//! process.

use super::{DesktopRuntime, DesktopRuntimeError};
use crate::pad_store::DesktopUiState;
use crate::permission_policy::{PermissionMode, TaskEnvironment};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::thread;
use std::time::Duration;

mod actions;
pub(super) mod format;
mod protocol;
pub(crate) mod remote_events;
use actions::auth_result;
pub(crate) use actions::handle_request;

const DESKTOP_PROTOCOL_VERSION: u32 = protocol::LEGACY_PROTOCOL_VERSION;

/// Stable request envelope consumed by the native macOS host.
///
/// Unknown fields are ignored deliberately so the WebView can add optional
/// presentation data without making older PAD binaries fail a request.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct DesktopRequest {
    #[serde(default)]
    pub id: Option<String>,
    pub action: Option<String>,
    #[serde(default)]
    pub protocol_version: Option<u32>,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub credential_ref: Option<String>,
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub environment: Option<TaskEnvironment>,
    #[serde(default)]
    pub permission_mode: Option<PermissionMode>,
    #[serde(default)]
    pub unattended: Option<bool>,
    #[serde(default)]
    pub pinned: Option<bool>,
    #[serde(default)]
    pub archived: Option<bool>,
    #[serde(default)]
    pub unread: Option<bool>,
    #[serde(default)]
    pub model_id: Option<String>,
    /// Compatibility aliases used by the Swift renderer.  The canonical
    /// bridge names remain `default_provider`/`model_id`, but accepting these
    /// aliases keeps an older bundled renderer interoperable with a newer
    /// host during app upgrades.
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub thinking_level: Option<String>,
    #[serde(default)]
    pub fast_mode: Option<bool>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub interaction_id: Option<String>,
    #[serde(default)]
    pub response_kind: Option<String>,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub auth_type: Option<String>,
    #[serde(default)]
    pub attempt_id: Option<String>,
    #[serde(default)]
    pub prompt_id: Option<String>,
    #[serde(default)]
    pub cancelled: Option<bool>,
    #[serde(default)]
    pub pane_id: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub columns: Option<u16>,
    #[serde(default)]
    pub rows: Option<u16>,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub state: Option<DesktopUiState>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub refresh: Option<bool>,
    #[serde(default)]
    pub pairing_id: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DesktopResponse {
    pub id: Option<String>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DesktopErrorBody>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DesktopErrorBody {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug)]
pub(crate) struct BridgeError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl BridgeError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for BridgeError {}

impl From<DesktopRuntimeError> for BridgeError {
    fn from(error: DesktopRuntimeError) -> Self {
        let code = match error {
            DesktopRuntimeError::Store(_) => "store_error",
            DesktopRuntimeError::Pi(_) => "pi_error",
            DesktopRuntimeError::TaskNotFound(_) => "task_not_found",
            DesktopRuntimeError::ProfileNotFound(_) => "profile_not_found",
            DesktopRuntimeError::ProjectNotFound(_) => "project_not_found",
            DesktopRuntimeError::ProfileMismatch { .. } => "profile_mismatch",
            DesktopRuntimeError::InvalidSessionPath { .. } => "invalid_session_path",
            DesktopRuntimeError::TaskAlreadyRunning(_) => "task_already_running",
            DesktopRuntimeError::DataRootLocked => "desktop_data_root_locked",
        };
        Self::new(code, error.to_string())
    }
}

/// Run the Desktop host over stdin/stdout.  No logger is initialized here and
/// no diagnostic is printed to stdout: stdout is reserved for response JSONL.
pub(crate) fn run_server() -> Result<(), Box<dyn Error>> {
    let mut runtime =
        DesktopRuntime::open_default().map_err(|error| io::Error::other(error.to_string()))?;
    let (input_tx, input_rx) =
        mpsc::sync_channel(crate::desktop_runtime::remote::OWNER_QUEUE_DEPTH);
    let stdin_tx = input_tx.clone();
    thread::spawn(move || read_server_input(io::BufReader::new(io::stdin()), stdin_tx));
    runtime.attach_remote_gateway(input_tx);
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let mut negotiated_v2 = false;
    let mut sequence = 0_u64;
    loop {
        match input_rx.recv_timeout(Duration::from_millis(40)) {
            Ok(ServerInput::Remote(request)) => {
                runtime.handle_remote_owner_request(request);
                remote_events::pump_runtime_tasks(
                    &mut runtime,
                    &mut stdout,
                    negotiated_v2,
                    &mut sequence,
                )?;
                stdout.flush()?;
            }
            Ok(ServerInput::RemoteStatusChanged) => {
                if negotiated_v2 {
                    if let Some(payload) = runtime.remote_changed_payload("connection_changed") {
                        sequence = sequence.saturating_add(1);
                        write_event_line(
                            &mut stdout,
                            &protocol::event_frame(sequence, "remote_changed", payload),
                        )?;
                        stdout.flush()?;
                    }
                }
            }
            Ok(ServerInput::Line(line)) => {
                if line.trim().is_empty() {
                    continue;
                }
                let request = serde_json::from_str::<DesktopRequest>(&line).ok();
                negotiated_v2 |= request.as_ref().is_some_and(|request| {
                    request.protocol_version == Some(protocol::CURRENT_PROTOCOL_VERSION)
                        || request.action.as_deref() == Some("hello")
                });
                let (response, should_stop) = handle_line(&mut runtime, &line);
                write_response_line(&mut stdout, &response)?;
                if let Some(request) = request.as_ref().filter(|_| response.ok) {
                    remote_events::publish_local_result_to_remote(&runtime, request, &response);
                }
                if negotiated_v2 {
                    if let Some(request) = request.as_ref() {
                        if protocol::request_version(request) == protocol::CURRENT_PROTOCOL_VERSION
                        {
                            for event in
                                events_after_request(&runtime, request, &response, &mut sequence)
                            {
                                write_event_line(&mut stdout, &event)?;
                            }
                        }
                    }
                }
                stdout.flush()?;
                remote_events::pump_runtime_tasks(
                    &mut runtime,
                    &mut stdout,
                    negotiated_v2,
                    &mut sequence,
                )?;
                stdout.flush()?;
                if should_stop {
                    break;
                }
            }
            Ok(ServerInput::FrameTooLarge) => {
                write_protocol_error(
                    &mut stdout,
                    "frame_too_large",
                    "Desktop request exceeds the protocol frame limit",
                )?;
            }
            Ok(ServerInput::InvalidUtf8) => {
                write_protocol_error(
                    &mut stdout,
                    "invalid_utf8",
                    "Desktop request must be valid UTF-8",
                )?;
            }
            Ok(ServerInput::IoError(message)) => {
                write_protocol_error(&mut stdout, "transport_error", &message)?;
                break;
            }
            Ok(ServerInput::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                if negotiated_v2 {
                    let (snapshot, changed) = runtime.auth_status();
                    if changed {
                        let mut payload = match auth_result(&runtime, snapshot) {
                            Ok(payload) => payload,
                            // An authentication helper may still be completing
                            // after the user switched accounts. Keep it owned by
                            // Rust, but never publish that account's state into
                            // the newly active renderer.
                            Err(error) if error.code == "profile_not_active" => Value::Null,
                            Err(error) => return Err(Box::new(error)),
                        };
                        if !payload.is_null() {
                            protocol::sanitize_v2_result(&runtime, &mut payload)?;
                            sequence = sequence.saturating_add(1);
                            write_event_line(
                                &mut stdout,
                                &protocol::event_frame(sequence, "auth_changed", payload.clone()),
                            )?;
                            sequence = sequence.saturating_add(1);
                            write_event_line(
                                &mut stdout,
                                &protocol::event_frame(sequence, "account_changed", payload),
                            )?;
                        }
                    }
                }
                remote_events::pump_runtime_tasks(
                    &mut runtime,
                    &mut stdout,
                    negotiated_v2,
                    &mut sequence,
                )?;
                stdout.flush()?;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct RemoteOwnerRequest {
    pub(crate) device_id: String,
    pub(crate) action: String,
    pub(crate) params: Value,
    pub(crate) response:
        std::sync::mpsc::Sender<crate::desktop_runtime::remote::RemoteCommandOutcome>,
}

#[derive(Debug)]
pub(crate) enum ServerInput {
    Line(String),
    FrameTooLarge,
    InvalidUtf8,
    IoError(String),
    Disconnected,
    Remote(RemoteOwnerRequest),
    RemoteStatusChanged,
}

impl PartialEq for ServerInput {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Line(left), Self::Line(right)) | (Self::IoError(left), Self::IoError(right)) => {
                left == right
            }
            (Self::FrameTooLarge, Self::FrameTooLarge)
            | (Self::InvalidUtf8, Self::InvalidUtf8)
            | (Self::Disconnected, Self::Disconnected) => true,
            (Self::RemoteStatusChanged, Self::RemoteStatusChanged) => true,
            _ => false,
        }
    }
}

impl Eq for ServerInput {}

fn read_server_input(reader: impl BufRead, sender: SyncSender<ServerInput>) {
    let mut reader = reader;
    loop {
        match read_bounded_frame(&mut reader) {
            Ok(Some(input)) => {
                let disconnected = matches!(input, ServerInput::Disconnected);
                if sender.send(input).is_err() || disconnected {
                    break;
                }
            }
            Ok(None) => {
                let _ = sender.send(ServerInput::Disconnected);
                break;
            }
            Err(error) => {
                let _ = sender.send(ServerInput::IoError(error.to_string()));
                break;
            }
        }
    }
}

fn read_bounded_frame(reader: &mut impl BufRead) -> io::Result<Option<ServerInput>> {
    let mut frame = Vec::new();
    let mut oversized = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if frame.is_empty() && !oversized {
                return Ok(None);
            }
            break;
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        let data_len = newline.unwrap_or(available.len());
        if !oversized {
            if frame.len().saturating_add(data_len) > protocol::MAX_DESKTOP_FRAME_BYTES {
                oversized = true;
                frame.clear();
            } else {
                frame.extend_from_slice(&available[..data_len]);
            }
        }
        reader.consume(consumed);
        if newline.is_some() {
            break;
        }
    }
    if oversized {
        return Ok(Some(ServerInput::FrameTooLarge));
    }
    while matches!(frame.last(), Some(b'\r')) {
        frame.pop();
    }
    match String::from_utf8(frame) {
        Ok(line) => Ok(Some(ServerInput::Line(line))),
        Err(_) => Ok(Some(ServerInput::InvalidUtf8)),
    }
}

fn write_encoded_line(writer: &mut impl Write, encoded: &[u8]) -> io::Result<()> {
    if encoded.len() > protocol::MAX_DESKTOP_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Desktop response exceeds the protocol frame limit",
        ));
    }
    writer.write_all(encoded)?;
    writer.write_all(b"\n")
}

fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    let encoded = serde_json::to_vec(value)?;
    write_encoded_line(writer, &encoded)
}

/// A large request result is replaced with a small correlated error rather
/// than letting one history response tear down the long-lived server.
fn write_response_line(writer: &mut impl Write, response: &DesktopResponse) -> io::Result<()> {
    let encoded = serde_json::to_vec(response)?;
    if encoded.len() <= protocol::MAX_DESKTOP_FRAME_BYTES {
        return write_encoded_line(writer, &encoded);
    }
    write_json_line(
        writer,
        &DesktopResponse {
            id: response.id.clone(),
            ok: false,
            result: None,
            error: Some(DesktopErrorBody {
                code: "response_too_large",
                message: "Desktop response exceeds the protocol frame limit".to_string(),
            }),
        },
    )
}

/// Server events are advisory invalidations. If a future event accidentally
/// grows beyond the negotiated frame bound, drop that event while preserving
/// the request/response stream and the poll compatibility path.
fn write_event_line(writer: &mut impl Write, event: &Value) -> io::Result<()> {
    let encoded = serde_json::to_vec(event)?;
    if encoded.len() > protocol::MAX_DESKTOP_FRAME_BYTES {
        return Ok(());
    }
    write_encoded_line(writer, &encoded)
}

fn write_protocol_error(
    writer: &mut impl Write,
    code: &'static str,
    message: &str,
) -> io::Result<()> {
    write_response_line(
        writer,
        &DesktopResponse {
            id: None,
            ok: false,
            result: None,
            error: Some(DesktopErrorBody {
                code,
                message: message.to_string(),
            }),
        },
    )?;
    writer.flush()
}

fn events_after_request(
    runtime: &DesktopRuntime,
    request: &DesktopRequest,
    response: &DesktopResponse,
    sequence: &mut u64,
) -> Vec<Value> {
    if !response.ok {
        return Vec::new();
    }
    let action = request.action.as_deref().unwrap_or("");
    let mut events = Vec::new();
    let mut push = |kind: &str, payload: Value| {
        *sequence = sequence.saturating_add(1);
        events.push(protocol::event_frame(*sequence, kind, payload));
    };
    if matches!(
        action,
        "create_task"
            | "start_task"
            | "retry_task"
            | "prompt"
            | "poll"
            | "abort"
            | "stop"
            | "stop_task"
            | "set_task"
    ) {
        let task = request.task_id.as_deref().and_then(|task_id| {
            protocol::safe_task_from_runtime(runtime, task_id)
                .ok()
                .flatten()
        });
        push(
            "task_changed",
            json!({ "action": action, "task_id": request.task_id, "task": task }),
        );
    }
    if matches!(
        action,
        "start_task"
            | "retry_task"
            | "prompt"
            | "poll"
            | "abort"
            | "runtime_snapshot"
            | "stop"
            | "stop_task"
    ) {
        push(
            "runtime_changed",
            json!({
                "action": action,
                "task_id": request.task_id,
                "runtime": response.result.as_ref().and_then(|result| result.get("runtime")).cloned(),
                "backend": response.result.as_ref().and_then(|result| result.get("backend")).cloned(),
            }),
        );
    }
    if matches!(action, "create_profile" | "set_profile" | "logout") {
        push(
            "account_changed",
            json!({
                "action": action,
                "profile_id": request.profile_id,
                "account": response.result.as_ref().and_then(|result| result.get("account")).cloned(),
            }),
        );
    }
    if matches!(
        action,
        "auth_begin" | "auth_status" | "auth_respond" | "auth_cancel" | "logout"
    ) {
        push(
            "auth_changed",
            response.result.clone().unwrap_or(Value::Null),
        );
    }
    if matches!(
        action,
        "remote_set_enabled" | "remote_pair_begin" | "remote_pair_cancel" | "remote_device_revoke"
    ) {
        if let Some(payload) = runtime.remote_changed_payload(action) {
            push("remote_changed", payload);
        }
    }
    events
}

pub(crate) fn handle_line(runtime: &mut DesktopRuntime, line: &str) -> (DesktopResponse, bool) {
    if line.len() > protocol::MAX_DESKTOP_FRAME_BYTES {
        return (
            DesktopResponse {
                id: None,
                ok: false,
                result: None,
                error: Some(DesktopErrorBody {
                    code: "frame_too_large",
                    message: "Desktop request exceeds the protocol frame limit".to_string(),
                }),
            },
            false,
        );
    }
    let raw = match serde_json::from_str::<Value>(line) {
        Ok(value) => value,
        Err(error) => {
            return (
                DesktopResponse {
                    id: None,
                    ok: false,
                    result: None,
                    error: Some(DesktopErrorBody {
                        code: "invalid_json",
                        message: error.to_string(),
                    }),
                },
                false,
            );
        }
    };
    let request = match serde_json::from_value::<DesktopRequest>(raw.clone()) {
        Ok(request) => request,
        Err(error) => {
            return (
                DesktopResponse {
                    id: None,
                    ok: false,
                    result: None,
                    error: Some(DesktopErrorBody {
                        code: "invalid_request",
                        message: error.to_string(),
                    }),
                },
                false,
            );
        }
    };
    let id = request.id.clone();
    if request
        .protocol_version
        .is_some_and(|version| !matches!(version, 1 | 2))
    {
        return (
            DesktopResponse {
                id,
                ok: false,
                result: None,
                error: Some(DesktopErrorBody {
                    code: "unsupported_protocol_version",
                    message: "PAD Desktop supports protocol versions 1 and 2".to_string(),
                }),
            },
            false,
        );
    }
    if request.protocol_version == Some(protocol::CURRENT_PROTOCOL_VERSION) {
        if let Err(error) = protocol::validate_v2_request(&raw, &request) {
            let message = protocol::sanitize_v2_error_message(runtime, &error.message);
            return (
                DesktopResponse {
                    id,
                    ok: false,
                    result: None,
                    error: Some(DesktopErrorBody {
                        code: error.code,
                        message,
                    }),
                },
                false,
            );
        }
    }
    if matches!(
        request.action.as_deref(),
        Some(
            "auth_begin"
                | "auth_status"
                | "auth_respond"
                | "auth_cancel"
                | "logout"
                | "terminal_open"
                | "terminal_input"
                | "terminal_resize"
                | "terminal_snapshot"
                | "terminal_close"
                | "get_ui_state"
                | "set_ui_state"
                | "remote_status"
                | "remote_set_enabled"
                | "remote_pair_begin"
                | "remote_pair_cancel"
                | "remote_device_revoke"
        )
    ) && protocol::request_version(&request) != protocol::CURRENT_PROTOCOL_VERSION
    {
        return (
            DesktopResponse {
                id,
                ok: false,
                result: None,
                error: Some(DesktopErrorBody {
                    code: "protocol_upgrade_required",
                    message: "this control-plane action requires Desktop protocol v2".to_string(),
                }),
            },
            false,
        );
    }
    let action = request.action.as_deref().unwrap_or("");
    if action == "shutdown" {
        return (
            DesktopResponse {
                id,
                ok: true,
                result: Some(json!({ "stopping": true })),
                error: None,
            },
            true,
        );
    }
    let result = handle_request(runtime, &request);
    match result {
        Ok(result) => (
            DesktopResponse {
                id,
                ok: true,
                result: Some(result),
                error: None,
            },
            false,
        ),
        Err(error) => {
            let message =
                if protocol::request_version(&request) == protocol::CURRENT_PROTOCOL_VERSION {
                    protocol::sanitize_v2_error_message(runtime, &error.message)
                } else {
                    error.message
                };
            (
                DesktopResponse {
                    id,
                    ok: false,
                    result: None,
                    error: Some(DesktopErrorBody {
                        code: error.code,
                        message,
                    }),
                },
                false,
            )
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
