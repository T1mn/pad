use super::super::format::{
    poll_has_provider_auth_error, poll_value, runtime_status_name, snapshot_value,
};
use super::super::{BridgeError, DesktopRequest};
use super::required;
use crate::desktop_runtime::{DesktopRuntime, DesktopRuntimeError};
use serde_json::{json, Value};

pub(super) fn start_task(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let generation = runtime.start_task(task_id).map_err(BridgeError::from)?;
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

pub(super) fn retry_task(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let generation = runtime.retry_task(task_id).map_err(BridgeError::from)?;
    Ok(json!({
        "task_id": task_id,
        "generation": generation,
        "running": true,
        "retrying": true,
    }))
}

pub(super) fn prompt(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let prompt = required(request.prompt.as_deref(), "prompt")?;
    let provider = request
        .default_provider
        .as_deref()
        .or(request.provider.as_deref());
    let model = request
        .model_id
        .as_deref()
        .or(request.default_model.as_deref())
        .or(request.model.as_deref());
    if provider.is_some() != model.is_some() {
        return Err(BridgeError::new(
            "invalid_model_selection",
            "provider and model must be supplied together",
        ));
    }
    let thinking_level = request
        .thinking_level
        .as_deref()
        .filter(|level| *level != "default");
    runtime
        .set_fast_mode(task_id, request.fast_mode.unwrap_or(true))
        .map_err(BridgeError::from)?;
    let started_here = match runtime.start_task_configured(task_id, provider, model, thinking_level)
    {
        Ok(_) => true,
        Err(DesktopRuntimeError::TaskAlreadyRunning(_)) => false,
        Err(error) => return Err(BridgeError::from(error)),
    };
    let result = (|| {
        // A freshly started Pi received these as native CLI options. Only an
        // already-live session needs an in-band model/thinking change.
        if !started_here {
            if let (Some(provider), Some(model)) = (provider, model) {
                runtime.set_model(task_id, provider, model)?;
            }
            if let Some(level) = thinking_level {
                runtime.set_thinking_level(task_id, level)?;
            }
        }
        runtime.send_prompt(task_id, prompt)
    })();
    if let Err(error) = result {
        if started_here {
            let _ = runtime.stop_task(task_id);
        }
        return Err(BridgeError::from(error));
    }
    Ok(json!({ "task_id": task_id, "accepted": true }))
}

pub(super) fn get_messages(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let response = runtime.get_messages(task_id).map_err(BridgeError::from)?;
    native_response(runtime, task_id, "get_messages", response)
}

pub(super) fn history(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let response = runtime.history(task_id).map_err(BridgeError::from)?;
    let pending = response
        .get("pending")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let messages = response
        .get("data")
        .and_then(|data| data.get("messages"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let mut result = native_response(runtime, task_id, "history", Some(response))?;
    if let Some(object) = result.as_object_mut() {
        object.insert("messages".to_string(), messages);
        object.insert("pending".to_string(), Value::Bool(pending));
    }
    Ok(result)
}

pub(super) fn get_state(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let response = runtime.get_state(task_id).map_err(BridgeError::from)?;
    native_response(runtime, task_id, "get_state", response)
}

pub(super) fn get_entries(
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

pub(super) fn set_model(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
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

pub(super) fn set_thinking_level(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let level = required(request.thinking_level.as_deref(), "thinking_level")?;
    runtime
        .set_thinking_level(task_id, level)
        .map_err(BridgeError::from)?;
    Ok(json!({ "task_id": task_id, "accepted": true, "thinking_level": level }))
}

pub(super) fn respond_ui(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    let request_id = required(
        request
            .request_id
            .as_deref()
            .or(request.interaction_id.as_deref()),
        "request_id",
    )?;
    let value = request.value.clone();
    let cancelled = request.cancelled.unwrap_or(false);
    if value.is_none() && !cancelled {
        return Err(BridgeError::new("invalid_request", "missing value"));
    }
    runtime
        .respond_ui(
            task_id,
            request_id,
            request.response_kind.as_deref(),
            value.clone(),
            cancelled,
        )
        .map_err(BridgeError::from)?;
    Ok(json!({
        "task_id": task_id,
        "request_id": request_id,
        "accepted": true,
        "value": value,
        "cancelled": cancelled,
    }))
}

pub(super) fn abort(
    runtime: &DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    runtime.abort_task(task_id).map_err(BridgeError::from)?;
    Ok(json!({ "task_id": task_id, "accepted": true }))
}

pub(super) fn poll(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
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

pub(super) fn runtime_snapshot(
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

pub(super) fn stop(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let task_id = required(request.task_id.as_deref(), "task_id")?;
    runtime.stop_task(task_id).map_err(BridgeError::from)?;
    Ok(json!({ "task_id": task_id, "stopped": true }))
}
