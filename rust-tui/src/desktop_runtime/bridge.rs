//! Line-oriented host bridge for the macOS Desktop shell.
//!
//! The bridge is intentionally transport-thin: Swift/WKWebView sends one
//! JSON request per stdin line and receives one JSON response per stdout
//! line.  All durable state and Pi process ownership stays in
//! `DesktopRuntime`; the renderer never receives a SQLite handle or child
//! process.

use super::{DesktopRuntime, DesktopRuntimeError};
use crate::permission_policy::{PermissionMode, PolicyLayer, Profile, Task, TaskEnvironment};
use crate::pi_runtime::{PiPoll, PiRuntimeSnapshot, PiRuntimeStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const DESKTOP_PROTOCOL_VERSION: u32 = 1;

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
    pub command: Option<String>,
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
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<DesktopRuntimeError> for BridgeError {
    fn from(error: DesktopRuntimeError) -> Self {
        let code = match error {
            DesktopRuntimeError::Store(_) => "store_error",
            DesktopRuntimeError::Pi(_) => "pi_error",
            DesktopRuntimeError::TaskNotFound(_) => "task_not_found",
            DesktopRuntimeError::ProfileNotFound(_) => "profile_not_found",
            DesktopRuntimeError::ProjectNotFound(_) => "project_not_found",
            DesktopRuntimeError::ProfileMismatch { .. } => "profile_mismatch",
            DesktopRuntimeError::TaskAlreadyRunning(_) => "task_already_running",
        };
        Self::new(code, error.to_string())
    }
}

/// Run the Desktop host over stdin/stdout.  No logger is initialized here and
/// no diagnostic is printed to stdout: stdout is reserved for response JSONL.
pub(crate) fn run_server() -> Result<(), Box<dyn Error>> {
    let mut runtime =
        DesktopRuntime::open_default().map_err(|error| io::Error::other(error.to_string()))?;
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let (response, should_stop) = handle_line(&mut runtime, &line);
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
        if should_stop {
            break;
        }
    }
    Ok(())
}

pub(crate) fn handle_line(runtime: &mut DesktopRuntime, line: &str) -> (DesktopResponse, bool) {
    let request = match serde_json::from_str::<DesktopRequest>(line) {
        Ok(request) => request,
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
    let id = request.id.clone();
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
        Err(error) => (
            DesktopResponse {
                id,
                ok: false,
                result: None,
                error: Some(DesktopErrorBody {
                    code: error.code,
                    message: error.message,
                }),
            },
            false,
        ),
    }
}

pub(crate) fn handle_request(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    match request.action.as_deref() {
        Some("ping") => Ok(json!({
            "protocol_version": DESKTOP_PROTOCOL_VERSION,
            "runtime": "pad-desktop",
            "pi": true,
        })),
        Some("bootstrap") => bootstrap(runtime),
        Some("list_sidebar") => sidebar(runtime),
        Some("create_profile") => create_profile(runtime, request),
        Some("create_task") => create_task(runtime, request),
        Some("start_task") => start_task(runtime, request),
        Some("prompt") => prompt(runtime, request),
        Some("poll") => poll(runtime, request),
        Some("runtime_snapshot") => runtime_snapshot(runtime, request),
        Some("stop") => stop(runtime, request),
        Some("stop_task") => stop(runtime, request),
        Some("set_task") => set_task(runtime, request),
        Some("set_profile") => set_profile(runtime, request),
        Some(_) | None => Err(BridgeError::new(
            "unknown_action",
            "action must be one of ping, bootstrap, list_sidebar, create_profile, create_task, start_task, prompt, poll, runtime_snapshot, stop_task, set_task, set_profile",
        )),
    }
}

fn bootstrap(runtime: &mut DesktopRuntime) -> Result<Value, BridgeError> {
    let profile = runtime
        .ensure_default_profile()
        .map_err(BridgeError::from)?;
    runtime
        .ensure_default_project(&profile.id)
        .map_err(BridgeError::from)?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    Ok(json!({
        "protocol_version": DESKTOP_PROTOCOL_VERSION,
        "profile": profile,
        "capabilities": [
            "profiles", "projects", "tasks", "codex_sidebar",
            "pi_rpc", "full_access_policy", "private_store"
        ],
        "sidebar": sidebar,
        "records": records,
    }))
}

fn sidebar(runtime: &DesktopRuntime) -> Result<Value, BridgeError> {
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    Ok(json!({ "sidebar": sidebar, "records": records }))
}

fn records_value(runtime: &DesktopRuntime) -> Result<Value, BridgeError> {
    let profiles = runtime
        .store()
        .list_profiles()
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let projects = runtime
        .store()
        .list_projects(true)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let tasks = runtime
        .store()
        .list_tasks(None, true)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    Ok(json!({ "profiles": profiles, "projects": projects, "tasks": tasks }))
}

fn create_profile(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let profile = Profile {
        id: request.profile_id.clone().unwrap_or_default(),
        name: request.name.clone().unwrap_or_default(),
        credential_ref: request.credential_ref.clone(),
        default_provider: request.default_provider.clone(),
        default_model: request.default_model.clone(),
        policy: PolicyLayer {
            mode: request.permission_mode.or(Some(PermissionMode::SystemFull)),
            unattended: request.unattended.or(Some(true)),
            protected_namespaces: crate::permission_policy::default_protected_namespaces(
                &dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            ),
            ..PolicyLayer::default()
        },
        ..Profile::default()
    };
    let profile = runtime.create_profile(profile).map_err(BridgeError::from)?;
    runtime
        .ensure_default_project(&profile.id)
        .map_err(BridgeError::from)?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    Ok(json!({ "profile": profile, "sidebar": sidebar, "records": records }))
}

fn create_task(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let profile_id = match request.profile_id.clone() {
        Some(id) if !id.trim().is_empty() => id,
        _ => runtime
            .store()
            .list_profiles()
            .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
            .into_iter()
            .next()
            .map(|profile| profile.id)
            .ok_or_else(|| {
                BridgeError::new("profile_not_found", "no PAD Desktop profile exists")
            })?,
    };
    let task = Task {
        id: request.task_id.clone().unwrap_or_default(),
        project_id: request.project_id.clone(),
        profile_id,
        title: request.title.clone().unwrap_or_default(),
        summary: request.summary.clone().unwrap_or_default(),
        cwd: request.cwd.clone().unwrap_or_default(),
        environment: request.environment.unwrap_or_default(),
        policy: PolicyLayer {
            mode: request.permission_mode,
            unattended: request.unattended,
            ..PolicyLayer::default()
        },
        ..Task::default()
    };
    let task = runtime.create_task(task).map_err(BridgeError::from)?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    Ok(json!({ "task": task, "sidebar": sidebar, "records": records }))
}

fn start_task(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let generation = runtime
        .start_task(task_id, request.command.as_deref().unwrap_or("pi"))
        .map_err(BridgeError::from)?;
    Ok(json!({
        "task_id": task_id,
        "generation": generation,
        "running": true,
    }))
}

fn prompt(runtime: &DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let prompt = required(request.prompt.as_deref(), "prompt")?;
    runtime
        .send_prompt(task_id, prompt)
        .map_err(BridgeError::from)?;
    Ok(json!({ "task_id": task_id, "accepted": true }))
}

fn poll(runtime: &mut DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let poll = runtime.poll_task(task_id).map_err(BridgeError::from)?;
    let snapshot = runtime
        .runtime_snapshot(task_id)
        .map_err(BridgeError::from)?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    Ok(json!({
        "task_id": task_id,
        "poll": poll_value(&poll),
        "runtime": snapshot.as_ref().map(snapshot_value),
        "sidebar": sidebar,
    }))
}

fn runtime_snapshot(
    runtime: &DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let snapshot = runtime
        .runtime_snapshot(task_id)
        .map_err(BridgeError::from)?;
    Ok(json!({
        "task_id": task_id,
        "runtime": snapshot.as_ref().map(snapshot_value),
    }))
}

fn stop(runtime: &mut DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    runtime.stop_task(task_id).map_err(BridgeError::from)?;
    Ok(json!({ "task_id": task_id, "stopped": true }))
}

fn set_task(runtime: &mut DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let mut task = runtime
        .store()
        .get_task(task_id)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
        .ok_or_else(|| {
            BridgeError::new(
                "task_not_found",
                format!("Desktop task '{task_id}' was not found"),
            )
        })?;
    if let Some(pinned) = request.pinned {
        task.pinned = pinned;
    }
    if let Some(archived) = request.archived {
        task.archived = archived;
    }
    if let Some(unread) = request.unread {
        task.unread = unread;
    }
    task.updated_at = super::unix_timestamp();
    runtime
        .store_mut()
        .update_task(&task)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    Ok(json!({ "task": task, "sidebar": sidebar, "records": records }))
}

fn set_profile(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let profile_id = required(request.profile_id.as_deref(), "profile_id")?;
    let profile = runtime
        .update_profile_policy(profile_id, request.permission_mode, request.unattended)
        .map_err(BridgeError::from)?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    Ok(json!({ "profile": profile, "sidebar": sidebar, "records": records }))
}

fn required<'a>(value: Option<&'a str>, field: &'static str) -> Result<&'a str, BridgeError> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| BridgeError::new("invalid_request", format!("missing {field}")))
}

fn poll_value(poll: &PiPoll) -> Value {
    let messages = poll
        .messages
        .iter()
        .map(|message| {
            json!({
                "type": message.message_type,
                "id": message.id,
                "value": message.value,
            })
        })
        .collect::<Vec<_>>();
    let events = poll
        .events
        .iter()
        .map(|event| event.value.clone())
        .collect::<Vec<_>>();
    let exit_status = poll.exit_status.map(|status| {
        json!({
            "code": status.code,
            "signal": status.signal,
            "killed": status.killed,
            "success": status.success(),
        })
    });
    json!({
        "messages": messages,
        "events": events,
        "stderr": String::from_utf8_lossy(&poll.stderr),
        "diagnostics": poll.diagnostics,
        "dropped_stale": poll.dropped_stale,
        "exit_status": exit_status,
    })
}

fn snapshot_value(snapshot: &PiRuntimeSnapshot) -> Value {
    json!({
        "generation": snapshot.generation,
        "status": runtime_status_name(snapshot.status),
        "pending_message_count": snapshot.pending_message_count,
        "active_tool_call_id": snapshot.active_tool_call_id,
        "last_sequence": snapshot.last_sequence,
    })
}

fn runtime_status_name(status: PiRuntimeStatus) -> &'static str {
    match status {
        PiRuntimeStatus::Starting => "starting",
        PiRuntimeStatus::Idle => "idle",
        PiRuntimeStatus::Running => "running",
        PiRuntimeStatus::Streaming => "streaming",
        PiRuntimeStatus::ToolRunning => "tool_running",
        PiRuntimeStatus::NeedsApproval => "needs_approval",
        PiRuntimeStatus::NeedsInput => "needs_input",
        PiRuntimeStatus::Compacting => "compacting",
        PiRuntimeStatus::Retrying => "retrying",
        PiRuntimeStatus::Failed => "failed",
        PiRuntimeStatus::Disconnected => "disconnected",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    fn runtime() -> DesktopRuntime {
        let mut runtime = DesktopRuntime::in_memory().unwrap();
        let profile = Profile {
            id: "profile-bridge".to_string(),
            name: "Bridge profile".to_string(),
            agent_dir: std::env::temp_dir().join("pad-bridge-agent"),
            session_dir: std::env::temp_dir().join("pad-bridge-sessions"),
            ..Profile::default()
        };
        runtime.store_mut().insert_profile(&profile).unwrap();
        runtime
    }

    #[test]
    fn protocol_returns_codex_sidebar_for_bootstrap() {
        let mut runtime = DesktopRuntime::in_memory().unwrap();
        let request: DesktopRequest =
            serde_json::from_str(r#"{"id":"b1","action":"bootstrap"}"#).unwrap();
        let result = handle_request(&mut runtime, &request).unwrap();
        assert_eq!(result["protocol_version"], DESKTOP_PROTOCOL_VERSION);
        assert_eq!(result["sidebar"]["view"], "all");
        assert_eq!(result["profile"]["id"], "default");
    }

    #[test]
    fn create_task_start_poll_and_stop_round_trip() {
        let mut runtime = runtime();
        let task_request: DesktopRequest = serde_json::from_value(json!({
            "action": "create_task",
            "profile_id": "profile-bridge",
            "task_id": "task-bridge",
            "title": "Bridge task",
            "cwd": std::env::temp_dir(),
        }))
        .unwrap();
        let result = handle_request(&mut runtime, &task_request).unwrap();
        assert_eq!(result["task"]["id"], "task-bridge");

        let start_request: DesktopRequest = serde_json::from_value(json!({
            "action": "start_task",
            "task_id": "task-bridge",
            "command": "/bin/sh -c 'printf \"%s\\n\" \"{\\\"type\\\":\\\"agent_settled\\\"}\"'",
        }))
        .unwrap();
        handle_request(&mut runtime, &start_request).unwrap();
        let poll_request: DesktopRequest = serde_json::from_value(json!({
            "action": "poll",
            "task_id": "task-bridge",
        }))
        .unwrap();
        let mut result = handle_request(&mut runtime, &poll_request).unwrap();
        for _ in 0..30 {
            if result["poll"]["events"]
                .as_array()
                .is_some_and(|events| !events.is_empty())
            {
                break;
            }
            thread::sleep(Duration::from_millis(5));
            result = handle_request(&mut runtime, &poll_request).unwrap();
        }
        assert!(result["poll"]["events"]
            .as_array()
            .is_some_and(|events| !events.is_empty()));
        let stop_request: DesktopRequest = serde_json::from_value(json!({
            "action": "stop_task",
            "task_id": "task-bridge",
        }))
        .unwrap();
        assert_eq!(
            handle_request(&mut runtime, &stop_request).unwrap()["stopped"],
            true
        );
        assert!(!runtime.is_running("task-bridge"));
        assert!(Path::new(".").exists());
    }

    #[test]
    fn malformed_input_is_one_error_response_and_does_not_write_stdout() {
        let mut runtime = runtime();
        let (response, stop) = handle_line(&mut runtime, "not json");
        assert!(!stop);
        assert!(!response.ok);
        assert_eq!(response.error.as_ref().unwrap().code, "invalid_json");
    }

    #[test]
    fn full_access_fields_are_persisted_on_profile_and_task_requests() {
        let mut runtime = DesktopRuntime::in_memory().unwrap();
        let profile_request: DesktopRequest = serde_json::from_value(json!({
            "action": "create_profile",
            "profile_id": "full",
            "name": "Full",
            "permission_mode": "system_full",
            "unattended": true,
        }))
        .unwrap();
        let profile_result = handle_request(&mut runtime, &profile_request).unwrap();
        assert_eq!(profile_result["profile"]["policy"]["mode"], "system_full");
        assert_eq!(profile_result["profile"]["policy"]["unattended"], true);
    }

    #[test]
    fn set_task_persists_sidebar_flags() {
        let mut runtime = runtime();
        let create_request: DesktopRequest = serde_json::from_value(json!({
            "action": "create_task",
            "profile_id": "profile-bridge",
            "task_id": "task-flags",
            "title": "Flagged task",
            "cwd": std::env::temp_dir(),
        }))
        .unwrap();
        handle_request(&mut runtime, &create_request).unwrap();

        let set_request: DesktopRequest = serde_json::from_value(json!({
            "action": "set_task",
            "task_id": "task-flags",
            "pinned": true,
            "archived": true,
        }))
        .unwrap();
        let result = handle_request(&mut runtime, &set_request).unwrap();
        assert_eq!(result["task"]["pinned"], true);
        assert_eq!(result["task"]["archived"], true);
        let stored = runtime.store().get_task("task-flags").unwrap().unwrap();
        assert!(stored.pinned);
        assert!(stored.archived);
    }

    #[test]
    fn set_profile_persists_policy() {
        let mut runtime = runtime();
        let request: DesktopRequest = serde_json::from_value(json!({
            "action": "set_profile",
            "profile_id": "profile-bridge",
            "permission_mode": "guarded",
            "unattended": false,
        }))
        .unwrap();

        let result = handle_request(&mut runtime, &request).unwrap();
        assert_eq!(result["profile"]["policy"]["mode"], "guarded");
        assert_eq!(result["profile"]["policy"]["unattended"], false);
        let stored = runtime
            .store()
            .get_profile("profile-bridge")
            .unwrap()
            .unwrap();
        assert_eq!(stored.policy.mode, Some(PermissionMode::Guarded));
        assert_eq!(stored.policy.unattended, Some(false));
    }

    #[test]
    fn response_shape_is_id_ok_result_or_error() {
        let mut runtime = runtime();
        let (response, _) = handle_line(&mut runtime, r#"{"id":"x","action":"ping"}"#);
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value["id"], "x");
        assert_eq!(value["ok"], true);
        assert!(value.get("result").is_some());
        assert!(value.get("error").is_none());
    }
}
