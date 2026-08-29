//! Line-oriented host bridge for the macOS Desktop shell.
//!
//! The bridge is intentionally transport-thin: Swift/WKWebView sends one
//! JSON request per stdin line and receives one JSON response per stdout
//! line.  All durable state and Pi process ownership stays in
//! `DesktopRuntime`; the renderer never receives a SQLite handle or child
//! process.

use super::{DesktopRuntime, DesktopRuntimeError};
use crate::permission_policy::{PermissionMode, PolicyLayer, Profile, Task, TaskEnvironment};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

mod format;
use format::{poll_has_provider_auth_error, poll_value, runtime_status_name, snapshot_value};

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
    pub request_id: Option<String>,
    #[serde(default)]
    pub interaction_id: Option<String>,
    #[serde(default)]
    pub response_kind: Option<String>,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub since: Option<String>,
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
            DesktopRuntimeError::InvalidSessionPath { .. } => "invalid_session_path",
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
        Some("create_project") => create_project(runtime, request),
        Some("create_task") => create_task(runtime, request),
        Some("start_task") => start_task(runtime, request),
        Some("retry_task") => retry_task(runtime, request),
        Some("prompt") => prompt(runtime, request),
        Some("poll") => poll(runtime, request),
        Some("history") => history(runtime, request),
        Some("get_messages") => get_messages(runtime, request),
        Some("get_state") => get_state(runtime, request),
        Some("get_entries") => get_entries(runtime, request),
        Some("set_model") => set_model(runtime, request),
        Some("set_thinking_level") => set_thinking_level(runtime, request),
        Some("respond_ui") => respond_ui(runtime, request),
        Some("extension_ui_response") => respond_ui(runtime, request),
        Some("provider_status") => provider_status(runtime, request),
        Some("abort") => abort(runtime, request),
        Some("runtime_snapshot") => runtime_snapshot(runtime, request),
        Some("stop") => stop(runtime, request),
        Some("stop_task") => stop(runtime, request),
        Some("set_task") => set_task(runtime, request),
        Some("set_profile") => set_profile(runtime, request),
        Some(_) | None => Err(BridgeError::new(
            "unknown_action",
            "action must be one of ping, bootstrap, list_sidebar, create_profile, create_project, create_task, start_task, retry_task, prompt, poll, history, get_messages, get_state, get_entries, set_model, set_thinking_level, respond_ui, extension_ui_response, provider_status, abort, runtime_snapshot, stop_task, set_task, set_profile",
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
    let authenticated_providers = runtime.authenticated_providers(&profile);
    Ok(json!({
        "protocol_version": DESKTOP_PROTOCOL_VERSION,
        "backend": {
            "status": "ready",
            "provider_authentication": runtime.provider_authentication_status(&profile),
            "authenticated_providers": authenticated_providers,
            "selected_provider": profile.default_provider,
            "selected_model": profile.default_model,
        },
        "profile": profile,
        "capabilities": [
            "profiles", "projects", "tasks", "codex_sidebar",
            "pi_rpc", "full_access_policy", "private_store",
            "history", "provider_status", "extension_ui_response",
            "set_model", "set_thinking_level", "create_project", "stop"
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
    let authenticated_providers = runtime.authenticated_providers(&profile);
    Ok(json!({
        "profile": profile,
        "authentication": {
            "status": runtime.provider_authentication_status(&profile),
            "authenticated_providers": authenticated_providers,
        },
        "sidebar": sidebar,
        "records": records,
    }))
}

fn create_project(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let profile_id = required(request.profile_id.as_deref(), "profile_id")?;
    let primary_root = request
        .cwd
        .clone()
        .ok_or_else(|| BridgeError::new("invalid_request", "missing cwd"))?;
    let project = runtime
        .create_project(
            profile_id,
            request.name.clone().unwrap_or_default(),
            primary_root,
        )
        .map_err(BridgeError::from)?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    Ok(json!({ "project": project, "sidebar": sidebar, "records": records }))
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
    let provider_authentication = runtime
        .store()
        .get_task(task_id)
        .ok()
        .flatten()
        .and_then(|task| runtime.store().get_profile(&task.profile_id).ok().flatten())
        .map(|profile| runtime.provider_authentication_status(&profile))
        .unwrap_or("unknown");
    Ok(json!({
        "task_id": task_id,
        "generation": generation,
        "running": true,
        "backend": {
            "status": "ready",
            "provider_authentication": provider_authentication,
            "task_runtime": "starting",
        },
    }))
}

fn retry_task(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let generation = runtime
        .retry_task(task_id, request.command.as_deref().unwrap_or("pi"))
        .map_err(BridgeError::from)?;
    Ok(json!({
        "task_id": task_id,
        "generation": generation,
        "running": true,
        "retrying": true,
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

fn get_messages(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let response = runtime.get_messages(task_id).map_err(BridgeError::from)?;
    native_response(runtime, task_id, "get_messages", response)
}

fn history(runtime: &mut DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let response = runtime.history(task_id).map_err(BridgeError::from)?;
    let messages = response
        .get("data")
        .and_then(|data| data.get("messages"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let mut result = native_response(runtime, task_id, "history", Some(response))?;
    if let Some(object) = result.as_object_mut() {
        object.insert("messages".to_string(), messages);
    }
    Ok(result)
}

fn get_state(runtime: &mut DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let response = runtime.get_state(task_id).map_err(BridgeError::from)?;
    native_response(runtime, task_id, "get_state", response)
}

fn get_entries(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let response = if let Some(since) = request.since.as_deref() {
        runtime
            .get_entries_since(task_id, since)
            .map_err(BridgeError::from)?
    } else {
        runtime.get_entries(task_id).map_err(BridgeError::from)?
    };
    native_response(runtime, task_id, "get_entries", response)
}

fn native_response(
    runtime: &DesktopRuntime,
    task_id: &str,
    command: &str,
    response: Option<Value>,
) -> Result<Value, BridgeError> {
    let task = runtime
        .store()
        .get_task(task_id)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    Ok(json!({
        "task_id": task_id,
        "command": command,
        "response": response,
        "pending": response.is_none(),
        "task": task,
        "sidebar": sidebar,
    }))
}

fn set_model(runtime: &DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let provider = required(
        request
            .default_provider
            .as_deref()
            .or(request.provider.as_deref()),
        "provider",
    )?;
    let model_id = required(
        request
            .model_id
            .as_deref()
            .or(request.default_model.as_deref())
            .or(request.model.as_deref()),
        "model_id",
    )?;
    runtime
        .set_model(task_id, provider, model_id)
        .map_err(BridgeError::from)?;
    Ok(json!({
        "task_id": task_id,
        "accepted": true,
        "provider": provider,
        "model_id": model_id,
    }))
}

fn set_thinking_level(
    runtime: &DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let level = required(request.thinking_level.as_deref(), "thinking_level")?;
    runtime
        .set_thinking_level(task_id, level)
        .map_err(BridgeError::from)?;
    Ok(json!({ "task_id": task_id, "accepted": true, "thinking_level": level }))
}

fn respond_ui(runtime: &DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let request_id = required(
        request
            .request_id
            .as_deref()
            .or(request.interaction_id.as_deref()),
        "request_id",
    )?;
    let value = request
        .value
        .clone()
        .ok_or_else(|| BridgeError::new("invalid_request", "missing value"))?;
    runtime
        .respond_ui(
            task_id,
            request_id,
            request.response_kind.as_deref(),
            value.clone(),
        )
        .map_err(BridgeError::from)?;
    Ok(json!({
        "task_id": task_id,
        "request_id": request_id,
        "accepted": true,
        "value": value,
    }))
}

fn provider_status(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let profile = if let Some(profile_id) = request.profile_id.as_deref() {
        runtime
            .store()
            .get_profile(profile_id)
            .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
            .ok_or_else(|| {
                BridgeError::new(
                    "profile_not_found",
                    format!("Desktop profile '{profile_id}' was not found"),
                )
            })?
    } else {
        runtime
            .store()
            .list_profiles()
            .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
            .into_iter()
            .next()
            .ok_or_else(|| BridgeError::new("profile_not_found", "no PAD Desktop profile exists"))?
    };
    Ok(json!({
        "profile_id": profile.id,
        "status": "ready",
        "provider_authentication": runtime.provider_authentication_status(&profile),
        "authenticated_providers": runtime.authenticated_providers(&profile),
        "selected_provider": profile.default_provider,
        "selected_model": profile.default_model,
    }))
}

fn abort(runtime: &DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    runtime.abort_task(task_id).map_err(BridgeError::from)?;
    Ok(json!({ "task_id": task_id, "accepted": true }))
}

fn poll(runtime: &mut DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let poll = runtime.poll_task(task_id).map_err(BridgeError::from)?;
    let snapshot = runtime
        .runtime_snapshot(task_id)
        .map_err(BridgeError::from)?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let task = runtime
        .store()
        .get_task(task_id)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let profile = task
        .as_ref()
        .and_then(|task| runtime.store().get_profile(&task.profile_id).ok().flatten());
    let provider_authentication = if poll_has_provider_auth_error(&poll) {
        "missing"
    } else {
        profile
            .as_ref()
            .map(|profile| runtime.provider_authentication_status(profile))
            .unwrap_or("unknown")
    };
    let task_runtime = snapshot
        .as_ref()
        .map(|snapshot| runtime_status_name(snapshot.status))
        .unwrap_or("unknown");
    Ok(json!({
        "task_id": task_id,
        "poll": poll_value(&poll),
        "runtime": snapshot.as_ref().map(snapshot_value),
        "task": task,
        "backend": {
            "status": "ready",
            "provider_authentication": provider_authentication,
            "task_runtime": task_runtime,
        },
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
    let mut profile = runtime
        .update_profile_policy(profile_id, request.permission_mode, request.unattended)
        .map_err(BridgeError::from)?;
    if request.default_provider.is_some()
        || request.default_model.is_some()
        || request.credential_ref.is_some()
    {
        profile = runtime
            .update_profile_settings(
                profile_id,
                request.default_provider.clone(),
                request.default_model.clone(),
                request.credential_ref.clone(),
            )
            .map_err(BridgeError::from)?;
    }
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    let authenticated_providers = runtime.authenticated_providers(&profile);
    Ok(json!({
        "profile": profile,
        "authentication": {
            "status": runtime.provider_authentication_status(&profile),
            "authenticated_providers": authenticated_providers,
        },
        "sidebar": sidebar,
        "records": records,
    }))
}

fn required<'a>(value: Option<&'a str>, field: &'static str) -> Result<&'a str, BridgeError> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| BridgeError::new("invalid_request", format!("missing {field}")))
}

#[cfg(test)]
pub(crate) mod tests;
