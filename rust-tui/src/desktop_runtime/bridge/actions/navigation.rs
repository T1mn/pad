use super::super::{protocol, BridgeError, DesktopRequest, DESKTOP_PROTOCOL_VERSION};
use super::{active_profile_id, required};
use crate::desktop_runtime::{DesktopRuntime, DesktopRuntimeError};
use crate::permission_policy::{PermissionMode, PolicyLayer, Profile, Task};
use serde_json::{json, Value};
use std::path::PathBuf;

pub(super) fn bootstrap(runtime: &mut DesktopRuntime) -> Result<Value, BridgeError> {
    let fallback_profile = runtime
        .ensure_default_profile()
        .map_err(BridgeError::from)?;
    let ui_state = runtime.desktop_ui_state().map_err(BridgeError::from)?;
    let profile = match ui_state.active_profile_id.as_deref() {
        Some(active_profile_id) => runtime
            .store()
            .get_profile(active_profile_id)
            .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
            .unwrap_or(fallback_profile),
        None => fallback_profile,
    };
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
        "ui_state": ui_state,
        "records": records,
    }))
}

pub(super) fn sidebar(runtime: &DesktopRuntime) -> Result<Value, BridgeError> {
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let ui_state = runtime.desktop_ui_state().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    Ok(json!({ "sidebar": sidebar, "ui_state": ui_state, "records": records }))
}

pub(super) fn get_ui_state(runtime: &DesktopRuntime) -> Result<Value, BridgeError> {
    let state = runtime.desktop_ui_state().map_err(BridgeError::from)?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    Ok(json!({ "state": state, "sidebar": sidebar }))
}

pub(super) fn set_ui_state(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let mut state = request
        .state
        .clone()
        .ok_or_else(|| BridgeError::new("invalid_request", "missing state"))?;
    let target_profile_id = match state.active_profile_id.as_deref() {
        Some(profile_id) => {
            let exists = runtime
                .store()
                .get_profile(profile_id)
                .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?
                .is_some();
            if !exists {
                return Err(BridgeError::new(
                    "profile_not_active",
                    "profile is unavailable for the active profile",
                ));
            }
            profile_id.to_string()
        }
        None => active_profile_id(runtime)?,
    };
    if let Some(task_id) = state.selected_task_id.as_deref() {
        let task = runtime
            .store()
            .get_task(task_id)
            .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
        if task.is_none_or(|task| task.profile_id != target_profile_id) {
            return Err(BridgeError::new(
                "task_not_found",
                "task is unavailable for the active profile",
            ));
        }
    }
    state.active_profile_id = Some(target_profile_id);
    let state = runtime
        .set_desktop_ui_state(state)
        .map_err(BridgeError::from)?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    Ok(json!({ "state": state, "sidebar": sidebar }))
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

pub(super) fn create_profile(
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

pub(super) fn create_project(
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

pub(super) fn create_task(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
    let profile_id = match request.profile_id.clone() {
        Some(id) if !id.trim().is_empty() => id,
        _ if protocol::request_version(request) == protocol::CURRENT_PROTOCOL_VERSION => {
            active_profile_id(runtime)?
        }
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

pub(super) fn set_task(
    runtime: &mut DesktopRuntime,
    request: &DesktopRequest,
) -> Result<Value, BridgeError> {
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
    task.updated_at = crate::desktop_runtime::unix_timestamp();
    runtime
        .store_mut()
        .update_task(&task)
        .map_err(|error| BridgeError::from(DesktopRuntimeError::Store(error)))?;
    let sidebar = runtime.sidebar_snapshot().map_err(BridgeError::from)?;
    let records = records_value(runtime)?;
    Ok(json!({ "task": task, "sidebar": sidebar, "records": records }))
}

pub(super) fn set_profile(
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
