use super::{protocol, BridgeError, DesktopRequest, DESKTOP_PROTOCOL_VERSION};
use crate::desktop_runtime::{DesktopRuntime, DesktopRuntimeError};
use crate::permission_policy::Task;
use serde_json::{json, Value};

mod account;
mod navigation;
mod task;

pub(super) use account::auth_result;
use account::{
    auth_begin, auth_cancel, auth_respond, auth_status, logout, provider_status, terminal_close,
    terminal_input, terminal_open, terminal_resize, terminal_snapshot,
};
use navigation::{
    bootstrap, create_profile, create_project, create_task, get_ui_state, set_profile, set_task,
    set_ui_state, sidebar,
};
use task::{
    abort, get_entries, get_messages, get_state, history, poll, prompt, respond_ui, retry_task,
    runtime_snapshot, set_model, set_thinking_level, start_task, stop,
};

fn model_catalog(runtime: &DesktopRuntime, request: &DesktopRequest) -> Result<Value, BridgeError> {
    let profile_id = required(request.profile_id.as_deref(), "profile_id")?;
    runtime
        .model_catalog(profile_id, request.refresh.unwrap_or(false))
        .map_err(|error| BridgeError::new(error.code(), error.message()))
}

pub(crate) fn handle_request(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    if protocol::request_version(request) == protocol::CURRENT_PROTOCOL_VERSION {
        authorize_v2_request(runtime, request)?;
    }
    let result = match request.action.as_deref() {
        Some("hello") => Ok(protocol::hello_value()),
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
        Some("model_catalog") => model_catalog(runtime, request),
        Some("abort") => abort(runtime, request),
        Some("runtime_snapshot") => runtime_snapshot(runtime, request),
        Some("stop") => stop(runtime, request),
        Some("stop_task") => stop(runtime, request),
        Some("set_task") => set_task(runtime, request),
        Some("set_profile") => set_profile(runtime, request),
        Some("auth_begin") => auth_begin(runtime, request),
        Some("auth_status") => auth_status(runtime, request),
        Some("auth_respond") => auth_respond(runtime, request),
        Some("auth_cancel") => auth_cancel(runtime, request),
        Some("logout") => logout(runtime, request),
        Some("terminal_open") => terminal_open(runtime, request),
        Some("terminal_input") => terminal_input(runtime, request),
        Some("terminal_resize") => terminal_resize(runtime, request),
        Some("terminal_snapshot") => terminal_snapshot(runtime, request),
        Some("terminal_close") => terminal_close(runtime, request),
        Some("get_ui_state") => get_ui_state(runtime),
        Some("set_ui_state") => set_ui_state(runtime, request),
        Some("remote_status") => Ok(runtime.remote_status()),
        Some("remote_set_enabled") => runtime.remote_set_enabled(
            request
                .enabled
                .ok_or_else(|| BridgeError::new("invalid_request", "missing enabled"))?,
        ),
        Some("remote_pair_begin") => runtime.remote_pair_begin(),
        Some("remote_pair_cancel") => {
            runtime.remote_pair_cancel(required(request.pairing_id.as_deref(), "pairing_id")?)
        }
        Some("remote_device_revoke") => {
            runtime.remote_device_revoke(required(request.device_id.as_deref(), "device_id")?)
        }
        Some(_) | None => Err(BridgeError::new(
            "unknown_action",
            "unsupported PAD Desktop action; call hello to discover protocol capabilities",
        )),
    }?;
    if protocol::request_version(request) == protocol::CURRENT_PROTOCOL_VERSION
        && request.action.as_deref() != Some("hello")
    {
        let mut safe = result;
        protocol::sanitize_v2_result(runtime, &mut safe)?;
        Ok(safe)
    } else {
        Ok(result)
    }
}

/// Enforce the renderer's active Profile at the Rust control-plane boundary.
///
/// The renderer is allowed to know the small list of safe Profile summaries
/// needed by the account switcher. Project/task/runtime identifiers and
/// controls are scoped to exactly one active Profile, even if a stale or
/// compromised renderer retained an identifier from an earlier account.
fn authorize_v2_request(
    runtime: &DesktopRuntime,
    request: &DesktopRequest,
) -> Result<(), BridgeError> {
    let action = request.action.as_deref().unwrap_or("");
    if matches!(
        action,
        "start_task"
            | "retry_task"
            | "prompt"
            | "poll"
            | "history"
            | "get_messages"
            | "get_state"
            | "get_entries"
            | "set_model"
            | "set_thinking_level"
            | "respond_ui"
            | "extension_ui_response"
            | "abort"
            | "runtime_snapshot"
            | "stop"
            | "stop_task"
            | "set_task"
            | "terminal_open"
    ) {
        let task_id = required(request.task_id.as_deref(), "task_id")?;
        require_active_task(runtime, task_id)?;
        return Ok(());
    }

    match action {
        "create_project" | "set_profile" | "auth_begin" | "logout" => {
            let profile_id = required(request.profile_id.as_deref(), "profile_id")?;
            require_active_profile(runtime, profile_id)?;
            if matches!(action, "auth_begin" | "logout") {
                require_active_running_auth(runtime)?;
            }
        }
        "provider_status" => {
            if let Some(profile_id) = request.profile_id.as_deref() {
                require_active_profile(runtime, profile_id)?;
            } else {
                active_profile_id(runtime)?;
            }
        }
        "model_catalog" => {
            let profile_id = required(request.profile_id.as_deref(), "profile_id")?;
            require_active_profile(runtime, profile_id)?;
        }
        "create_task" => authorize_v2_task_creation(runtime, request)?,
        "auth_status" | "auth_respond" | "auth_cancel" => {
            if let Some(profile_id) = request.profile_id.as_deref() {
                require_active_profile(runtime, profile_id)?;
            }
            require_active_auth_owner(runtime)?;
        }
        _ => {}
    }
    Ok(())
}

fn active_profile_id(runtime: &DesktopRuntime) -> Result<String, BridgeError> {
    protocol::active_profile_id(runtime)?.ok_or_else(|| {
        BridgeError::new(
            "profile_not_active",
            "profile is unavailable for the active profile",
        )
    })
}

fn require_active_profile(runtime: &DesktopRuntime, profile_id: &str) -> Result<(), BridgeError> {
    if active_profile_id(runtime)? != profile_id {
        return Err(BridgeError::new(
            "profile_not_active",
            "profile is unavailable for the active profile",
        ));
    }
    Ok(())
}

fn require_active_auth_owner(runtime: &DesktopRuntime) -> Result<(), BridgeError> {
    let active_profile_id = active_profile_id(runtime)?;
    if runtime
        .auth_owner_profile_id()
        .is_some_and(|owner| owner != active_profile_id.as_str())
    {
        return Err(BridgeError::new(
            "profile_not_active",
            "profile is unavailable for the active profile",
        ));
    }
    Ok(())
}

fn require_active_running_auth(runtime: &DesktopRuntime) -> Result<(), BridgeError> {
    let active_profile_id = active_profile_id(runtime)?;
    if runtime
        .auth_running_profile_id()
        .is_some_and(|owner| owner != active_profile_id.as_str())
    {
        return Err(BridgeError::new(
            "profile_not_active",
            "profile is unavailable for the active profile",
        ));
    }
    Ok(())
}

fn require_active_task(runtime: &DesktopRuntime, task_id: &str) -> Result<Task, BridgeError> {
    let active_profile_id = active_profile_id(runtime)?;
    let task = runtime
        .store()
        .get_task(task_id)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    task.filter(|task| task.profile_id == active_profile_id)
        .ok_or_else(|| {
            BridgeError::new(
                "task_not_found",
                "task is unavailable for the active profile",
            )
        })
}

fn require_active_project(runtime: &DesktopRuntime, project_id: &str) -> Result<(), BridgeError> {
    let active_profile_id = active_profile_id(runtime)?;
    let project = runtime
        .store()
        .get_project(project_id)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    if project.is_none_or(|project| project.profile_id.as_deref() != Some(&active_profile_id)) {
        return Err(BridgeError::new(
            "project_not_found",
            "project is unavailable for the active profile",
        ));
    }
    Ok(())
}

fn authorize_v2_task_creation(
    runtime: &DesktopRuntime,
    request: &DesktopRequest,
) -> Result<(), BridgeError> {
    let active_profile_id = active_profile_id(runtime)?;
    if request
        .profile_id
        .as_deref()
        .is_some_and(|profile_id| profile_id != active_profile_id)
    {
        return Err(BridgeError::new(
            "profile_not_active",
            "profile is unavailable for the active profile",
        ));
    }
    if let Some(project_id) = request.project_id.as_deref() {
        require_active_project(runtime, project_id)?;
    }
    if let Some(task_id) = request
        .task_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
    {
        if runtime
            .store()
            .get_task(task_id)
            .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
            .is_some()
        {
            return Err(BridgeError::new(
                "task_id_unavailable",
                "task_id is unavailable",
            ));
        }
    }
    Ok(())
}

fn required<'a>(value: Option<&'a str>, field: &'static str) -> Result<&'a str, BridgeError> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| BridgeError::new("invalid_request", format!("missing {field}")))
}
