use super::super::{protocol, BridgeError, DesktopRequest};
use super::{active_profile_id, require_active_profile, required};
use crate::desktop_runtime::{DesktopRuntime, DesktopRuntimeError};
use serde_json::{json, Value};

pub(super) fn provider_status(
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
        let profile_id = if protocol::request_version(request) == protocol::CURRENT_PROTOCOL_VERSION
        {
            active_profile_id(runtime)?
        } else {
            runtime
                .store()
                .list_profiles()
                .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
                .into_iter()
                .next()
                .map(|profile| profile.id)
                .ok_or_else(|| {
                    BridgeError::new("profile_not_found", "no PAD Desktop profile exists")
                })?
        };
        runtime
            .store()
            .get_profile(&profile_id)
            .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
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

pub(super) fn auth_begin(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let profile_id = required(request.profile_id.as_deref(), "profile_id")?;
    let provider = required(request.provider.as_deref(), "provider")?;
    let auth_type = required(request.auth_type.as_deref(), "auth_type")?;
    let snapshot = runtime
        .auth_begin(profile_id, provider, auth_type)
        .map_err(|error| BridgeError::new(error.code, error.message))?;
    auth_result(runtime, snapshot)
}

pub(super) fn auth_status(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    if let Some(profile_id) = request.profile_id.as_deref() {
        require_active_profile(runtime, profile_id)?;
    }
    let (snapshot, _) = runtime.auth_status();
    require_active_auth_snapshot(runtime, &snapshot)?;
    if let Some(attempt_id) = request.attempt_id.as_deref() {
        if snapshot.attempt_id.as_deref() != Some(attempt_id) {
            return Err(BridgeError::new(
                "auth_attempt_mismatch",
                "attempt_id does not match the current authentication operation",
            ));
        }
    }
    if let Some(profile_id) = request.profile_id.as_deref() {
        if snapshot
            .profile_id
            .as_deref()
            .is_some_and(|id| id != profile_id)
        {
            return Err(BridgeError::new(
                "auth_profile_mismatch",
                "authentication operation belongs to another profile",
            ));
        }
    }
    auth_result(runtime, snapshot)
}

pub(super) fn auth_respond(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let attempt_id = required(request.attempt_id.as_deref(), "attempt_id")?;
    let prompt_id = required(request.prompt_id.as_deref(), "prompt_id")?;
    let cancelled = request.cancelled.unwrap_or(false);
    let value = match request.value.clone() {
        Some(value) => value,
        None if cancelled => Value::Null,
        None => return Err(BridgeError::new("invalid_request", "missing value")),
    };
    let (current, _) = runtime.auth_status();
    require_active_auth_snapshot(runtime, &current)?;
    let snapshot = runtime
        .auth_respond(attempt_id, prompt_id, value, cancelled)
        .map_err(|error| BridgeError::new(error.code, error.message))?;
    auth_result(runtime, snapshot)
}

pub(super) fn auth_cancel(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let attempt_id = required(request.attempt_id.as_deref(), "attempt_id")?;
    let (current, _) = runtime.auth_status();
    require_active_auth_snapshot(runtime, &current)?;
    let snapshot = runtime
        .auth_cancel(attempt_id)
        .map_err(|error| BridgeError::new(error.code, error.message))?;
    auth_result(runtime, snapshot)
}

pub(super) fn logout(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let profile_id = required(request.profile_id.as_deref(), "profile_id")?;
    let provider = required(request.provider.as_deref(), "provider")?;
    let snapshot = runtime
        .logout(profile_id, provider)
        .map_err(|error| BridgeError::new(error.code, error.message))?;
    auth_result(runtime, snapshot)
}

pub(super) fn terminal_open(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let result = runtime
        .terminal_open(
            task_id,
            request.pane_id.as_deref(),
            request.label.as_deref(),
            request.columns,
            request.rows,
        )
        .map_err(|error| BridgeError::new(error.code, error.message))?;
    serde_json::to_value(result)
        .map_err(|error| BridgeError::new("serialization_error", error.to_string()))
}

pub(super) fn terminal_input(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let pane_id = required(request.pane_id.as_deref(), "pane_id")?;
    let data = request
        .data
        .as_deref()
        .ok_or_else(|| BridgeError::new("invalid_request", "missing data"))?;
    let result = runtime
        .terminal_input(pane_id, data)
        .map_err(|error| BridgeError::new(error.code, error.message))?;
    serde_json::to_value(result)
        .map_err(|error| BridgeError::new("serialization_error", error.to_string()))
}

pub(super) fn terminal_resize(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let pane_id = required(request.pane_id.as_deref(), "pane_id")?;
    let columns = request
        .columns
        .ok_or_else(|| BridgeError::new("invalid_request", "missing columns"))?;
    let rows = request
        .rows
        .ok_or_else(|| BridgeError::new("invalid_request", "missing rows"))?;
    let result = runtime
        .terminal_resize(pane_id, columns, rows)
        .map_err(|error| BridgeError::new(error.code, error.message))?;
    serde_json::to_value(result)
        .map_err(|error| BridgeError::new("serialization_error", error.to_string()))
}

pub(super) fn terminal_snapshot(
    runtime: &DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let pane_id = required(request.pane_id.as_deref(), "pane_id")?;
    let result = runtime
        .terminal_snapshot(pane_id)
        .map_err(|error| BridgeError::new(error.code, error.message))?;
    serde_json::to_value(result)
        .map_err(|error| BridgeError::new("serialization_error", error.to_string()))
}

pub(super) fn terminal_close(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let pane_id = required(request.pane_id.as_deref(), "pane_id")?;
    let result = runtime
        .terminal_close(pane_id)
        .map_err(|error| BridgeError::new(error.code, error.message))?;
    serde_json::to_value(result)
        .map_err(|error| BridgeError::new("serialization_error", error.to_string()))
}

pub(in crate::desktop_runtime::bridge) fn auth_result(
    runtime: &DesktopRuntime,
    snapshot: crate::desktop_runtime::auth::AuthSnapshot,
) -> Result<Value, BridgeError> {
    require_active_auth_snapshot(runtime, &snapshot)?;
    let account = snapshot.profile_id.as_deref().and_then(|profile_id| {
        runtime
            .store()
            .get_profile(profile_id)
            .ok()
            .flatten()
            .map(|profile| {
                json!({
                    "profile": profile,
                    "provider_authentication": runtime.provider_authentication_status(&profile),
                    "authenticated_providers": runtime.authenticated_providers(&profile),
                })
            })
    });
    Ok(json!({ "auth": snapshot, "account": account }))
}

fn require_active_auth_snapshot(
    runtime: &DesktopRuntime,
    snapshot: &crate::desktop_runtime::auth::AuthSnapshot,
) -> Result<(), BridgeError> {
    if let Some(profile_id) = snapshot.profile_id.as_deref() {
        require_active_profile(runtime, profile_id)?;
    }
    Ok(())
}
