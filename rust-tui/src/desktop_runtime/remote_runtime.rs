use super::bridge::{BridgeError, RemoteOwnerRequest, ServerInput};
use super::remote::{
    project_remote_result, remote_action_allowed, remote_action_mutates, RemoteCommandOutcome,
    RemoteGateway,
};
use super::{DesktopRuntime, DesktopRuntimeError};
use crate::permission_policy::{PolicyLayer, Task};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::sync::mpsc::SyncSender;

impl DesktopRuntime {
    pub(crate) fn attach_remote_gateway(&mut self, owner: SyncSender<ServerInput>) {
        match RemoteGateway::start(&self.data_root, owner) {
            Ok(gateway) => {
                self.remote_gateway = Some(gateway);
                self.remote_start_error = None;
            }
            Err(_) => {
                self.remote_gateway = None;
                self.remote_start_error = Some("remote_gateway_unavailable".to_string());
            }
        }
    }

    pub(crate) fn remote_status(&self) -> Value {
        let profile_id = self.active_remote_profile_id();
        self.remote_gateway.as_ref().map_or_else(
            || unavailable_status(self.remote_start_error.as_deref()),
            |gateway| gateway.status_value(profile_id.as_deref()),
        )
    }

    pub(crate) fn remote_set_enabled(&mut self, enabled: bool) -> Result<Value, BridgeError> {
        let profile_id = self.active_remote_profile_id();
        self.remote_gateway_mut()?
            .set_enabled(enabled, profile_id.as_deref())
            .map_err(|_| {
                BridgeError::new(
                    "remote_persistence_failed",
                    "remote setting could not be saved",
                )
            })
    }

    pub(crate) fn remote_pair_begin(&mut self) -> Result<Value, BridgeError> {
        let profile_id = self
            .desktop_ui_state()
            .map_err(BridgeError::from)?
            .active_profile_id
            .ok_or_else(|| {
                BridgeError::new("profile_not_active", "select an account before pairing")
            })?;
        self.remote_gateway_mut()?
            .pair_begin(&profile_id)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotConnected => {
                    BridgeError::new("remote_disabled", "enable remote access before pairing")
                }
                _ => BridgeError::new("remote_pairing_failed", "pairing could not be started"),
            })
    }

    pub(crate) fn remote_pair_cancel(&mut self, pairing_id: &str) -> Result<Value, BridgeError> {
        let profile_id = self.require_active_remote_profile_id()?;
        self.remote_gateway_mut()?
            .pair_cancel(pairing_id, &profile_id)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => {
                    BridgeError::new("remote_pairing_not_found", "pairing was not found")
                }
                _ => BridgeError::new("remote_pairing_failed", "pairing could not be cancelled"),
            })
    }

    pub(crate) fn remote_device_revoke(&mut self, device_id: &str) -> Result<Value, BridgeError> {
        let profile_id = self.require_active_remote_profile_id()?;
        self.remote_gateway_mut()?
            .revoke_device(device_id, &profile_id)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => {
                    BridgeError::new("remote_device_not_found", "remote device was not found")
                }
                _ => BridgeError::new(
                    "remote_persistence_failed",
                    "remote device could not be revoked",
                ),
            })
    }

    pub(crate) fn remote_changed_payload(&self, action: &str) -> Option<Value> {
        let profile_id = self.active_remote_profile_id();
        self.remote_gateway
            .as_ref()
            .map(|gateway| gateway.remote_changed(action, profile_id.as_deref()))
    }

    pub(crate) fn publish_remote_invalidation(&self, action: &str, task_id: Option<&str>) {
        if let Some(task_id) = task_id {
            self.publish_remote_task_event(
                task_id,
                "invalidated",
                json!({"action":action,"task_id":task_id}),
            );
        }
    }

    pub(crate) fn publish_remote_task_event(&self, task_id: &str, kind: &str, payload: Value) {
        let profile_id = self
            .store
            .get_task(task_id)
            .ok()
            .flatten()
            .map(|task| task.profile_id);
        if let (Some(gateway), Some(profile_id)) = (&self.remote_gateway, profile_id) {
            gateway.publish_profile_event(&profile_id, kind, project_remote_result(payload));
        }
    }

    pub(crate) fn active_task_ids(&self) -> Vec<String> {
        self.active_tasks.keys().cloned().collect()
    }

    pub(crate) fn handle_remote_owner_request(&mut self, request: RemoteOwnerRequest) {
        let outcome =
            self.execute_remote_command(&request.device_id, &request.action, request.params);
        let _ = request.response.send(outcome);
    }

    fn execute_remote_command(
        &mut self,
        device_id: &str,
        action: &str,
        params: Value,
    ) -> RemoteCommandOutcome {
        if !remote_action_allowed(action) {
            return RemoteCommandOutcome::rejected(
                "remote_action_denied",
                "action is not available over PAD Remote",
            );
        }
        if self
            .remote_gateway
            .as_ref()
            .is_none_or(|gateway| !gateway.is_enabled())
        {
            return RemoteCommandOutcome::rejected("remote_disabled", "remote access is disabled");
        }
        let Some(profile_id) = self
            .remote_gateway
            .as_ref()
            .and_then(|gateway| gateway.device_profile(device_id))
        else {
            return RemoteCommandOutcome::rejected(
                "remote_device_revoked",
                "remote device is unavailable",
            );
        };
        let result = self.execute_profile_remote_action(&profile_id, action, params);
        match result {
            Ok(result) => {
                if remote_action_mutates(action) {
                    self.publish_remote_invalidation(
                        action,
                        result.get("task_id").and_then(Value::as_str),
                    );
                }
                RemoteCommandOutcome {
                    ok: true,
                    result: Some(project_remote_result(result)),
                    error: None,
                }
            }
            Err(error) => {
                RemoteCommandOutcome::rejected(error.code, &safe_remote_error(error.code))
            }
        }
    }

    fn execute_profile_remote_action(
        &mut self,
        profile_id: &str,
        action: &str,
        params: Value,
    ) -> Result<Value, BridgeError> {
        self.require_remote_profile(profile_id)?;
        let object = params
            .as_object()
            .ok_or_else(|| BridgeError::new("invalid_request", "params must be an object"))?;
        validate_remote_fields(action, object)?;
        match action {
            "hello" => Ok(json!({
                "protocol": "pad.remote.v1",
                "capabilities": ["tasks", "history", "commands", "events", "receipts"],
            })),
            "bootstrap" | "list_sidebar" => self.remote_navigation(profile_id),
            "history" => {
                let task_id = required_string(object, "task_id")?;
                self.require_remote_task(profile_id, task_id)?;
                let response = self
                    .remote_history_response(task_id)
                    .map_err(BridgeError::from)?;
                let task = self.store.get_task(task_id).map_err(store_error)?;
                let mut result = json!({
                    "task_id": task_id,
                    "command": "history",
                    "pending": response.is_none(),
                    "task": task,
                    "sidebar": self.remote_sidebar(profile_id)?,
                    "pending_ui_requests": self.pending_ui_requests(task_id),
                });
                if let Some(response) = response {
                    let messages = response
                        .get("data")
                        .and_then(|value| value.get("messages"))
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    result["response"] = response;
                    result["messages"] = messages;
                }
                Ok(result)
            }
            "create_task" => self.remote_create_task(profile_id, object),
            "start_task" => {
                let task_id = self.remote_task_id(profile_id, object)?;
                let generation = self.start_task(task_id).map_err(BridgeError::from)?;
                Ok(json!({"task_id":task_id,"generation":generation,"running":true}))
            }
            "retry_task" => {
                let task_id = self.remote_task_id(profile_id, object)?;
                let generation = self.retry_task(task_id).map_err(BridgeError::from)?;
                Ok(
                    json!({"task_id":task_id,"generation":generation,"running":true,"retrying":true}),
                )
            }
            "prompt" => {
                let task_id = self.remote_task_id(profile_id, object)?;
                let prompt = required_string(object, "prompt")?;
                match self.start_task(task_id) {
                    Ok(_) | Err(DesktopRuntimeError::TaskAlreadyRunning(_)) => {}
                    Err(error) => return Err(BridgeError::from(error)),
                }
                self.send_prompt(task_id, prompt)
                    .map_err(BridgeError::from)?;
                Ok(json!({"task_id":task_id,"accepted":true}))
            }
            "abort" => {
                let task_id = self.remote_task_id(profile_id, object)?;
                self.abort_task(task_id).map_err(BridgeError::from)?;
                Ok(json!({"task_id":task_id,"accepted":true}))
            }
            "stop" | "stop_task" => {
                let task_id = self.remote_task_id(profile_id, object)?;
                self.stop_task(task_id).map_err(BridgeError::from)?;
                Ok(json!({"task_id":task_id,"stopped":true}))
            }
            "runtime_snapshot" => {
                let task_id = self.remote_task_id(profile_id, object)?;
                let runtime = self.runtime_snapshot(task_id).map_err(BridgeError::from)?;
                Ok(json!({
                    "task_id":task_id,
                    "runtime":runtime.map(remote_snapshot),
                    "pending_ui_requests":self.pending_ui_requests(task_id),
                }))
            }
            "respond_ui" => self.remote_respond_ui(profile_id, object),
            "set_task" => self.remote_set_task(profile_id, object),
            _ => Err(BridgeError::new(
                "remote_action_denied",
                "remote action is denied",
            )),
        }
    }

    #[cfg(test)]
    pub(crate) fn execute_profile_remote_action_for_test(
        &mut self,
        profile_id: &str,
        action: &str,
        params: Value,
    ) -> Result<Value, BridgeError> {
        self.execute_profile_remote_action(profile_id, action, params)
    }

    #[cfg(test)]
    pub(crate) fn execute_remote_command_for_test(
        &mut self,
        device_id: &str,
        action: &str,
        params: Value,
    ) -> RemoteCommandOutcome {
        self.execute_remote_command(device_id, action, params)
    }

    fn remote_navigation(&self, profile_id: &str) -> Result<Value, BridgeError> {
        let profile = self.store.get_profile(profile_id).map_err(store_error)?;
        let projects = self
            .store
            .list_projects(true)
            .map_err(store_error)?
            .into_iter()
            .filter(|project| project.profile_id.as_deref() == Some(profile_id))
            .collect::<Vec<_>>();
        let tasks = self
            .store
            .list_tasks(None, true)
            .map_err(store_error)?
            .into_iter()
            .filter(|task| task.profile_id == profile_id)
            .collect::<Vec<_>>();
        let pending_ui_requests_by_task = tasks
            .iter()
            .filter_map(|task| {
                let pending = self.pending_ui_requests(&task.id);
                (!pending.is_empty()).then(|| (task.id.clone(), json!(pending)))
            })
            .collect::<Map<String, Value>>();
        Ok(json!({
            "sidebar": self.remote_sidebar(profile_id)?,
            "records": {"profiles": profile.into_iter().collect::<Vec<_>>(), "projects": projects, "tasks": tasks},
            "pending_ui_requests_by_task": pending_ui_requests_by_task,
        }))
    }

    fn remote_create_task(
        &mut self,
        profile_id: &str,
        object: &Map<String, Value>,
    ) -> Result<Value, BridgeError> {
        let project_id = object.get("project_id").and_then(Value::as_str);
        if let Some(project_id) = project_id {
            self.require_remote_project(profile_id, project_id)?;
        }
        let task = Task {
            id: optional_string(object, "task_id").unwrap_or_default(),
            project_id: project_id.map(str::to_string),
            profile_id: profile_id.to_string(),
            title: optional_string(object, "title").unwrap_or_default(),
            summary: optional_string(object, "summary").unwrap_or_default(),
            policy: PolicyLayer::default(),
            ..Task::default()
        };
        let task = self.create_task(task).map_err(BridgeError::from)?;
        Ok(json!({"task_id":task.id,"task":task,"sidebar":self.remote_sidebar(profile_id)?}))
    }

    fn remote_respond_ui(
        &mut self,
        profile_id: &str,
        object: &Map<String, Value>,
    ) -> Result<Value, BridgeError> {
        let task_id = self.remote_task_id(profile_id, object)?;
        let request_id = optional_string(object, "request_id")
            .or_else(|| optional_string(object, "interaction_id"))
            .ok_or_else(|| BridgeError::new("invalid_request", "missing request_id"))?;
        let value = object.get("value").cloned();
        let cancelled = object
            .get("cancelled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if value.is_none() && !cancelled {
            return Err(BridgeError::new("invalid_request", "missing value"));
        }
        self.respond_ui(
            task_id,
            &request_id,
            object.get("response_kind").and_then(Value::as_str),
            value,
            cancelled,
        )
        .map_err(BridgeError::from)?;
        Ok(json!({
            "task_id":task_id,"request_id":request_id,
            "accepted":true,"cancelled":cancelled,
        }))
    }

    fn remote_set_task(
        &mut self,
        profile_id: &str,
        object: &Map<String, Value>,
    ) -> Result<Value, BridgeError> {
        let task_id = self.remote_task_id(profile_id, object)?;
        let mut task = self
            .store
            .get_task(task_id)
            .map_err(store_error)?
            .ok_or_else(|| BridgeError::new("task_not_found", "task is unavailable"))?;
        if let Some(value) = object.get("pinned").and_then(Value::as_bool) {
            task.pinned = value;
        }
        if let Some(value) = object.get("archived").and_then(Value::as_bool) {
            task.archived = value;
        }
        if let Some(value) = object.get("unread").and_then(Value::as_bool) {
            task.unread = value;
        }
        task.updated_at = super::unix_timestamp();
        self.store_mut().update_task(&task).map_err(store_error)?;
        Ok(json!({"task_id":task_id,"task":task,"sidebar":self.remote_sidebar(profile_id)?}))
    }

    fn remote_task_id<'a>(
        &self,
        profile_id: &str,
        object: &'a Map<String, Value>,
    ) -> Result<&'a str, BridgeError> {
        let task_id = required_string(object, "task_id")?;
        self.require_remote_task(profile_id, task_id)?;
        Ok(task_id)
    }

    fn require_remote_profile(&self, profile_id: &str) -> Result<(), BridgeError> {
        if self
            .store
            .get_profile(profile_id)
            .map_err(store_error)?
            .is_none()
        {
            return Err(BridgeError::new(
                "profile_unavailable",
                "paired account is unavailable",
            ));
        }
        Ok(())
    }

    fn require_remote_project(
        &self,
        profile_id: &str,
        project_id: &str,
    ) -> Result<(), BridgeError> {
        let project = self.store.get_project(project_id).map_err(store_error)?;
        if project.is_none_or(|project| project.profile_id.as_deref() != Some(profile_id)) {
            return Err(BridgeError::new(
                "project_not_found",
                "project is unavailable",
            ));
        }
        Ok(())
    }

    fn require_remote_task(&self, profile_id: &str, task_id: &str) -> Result<(), BridgeError> {
        let task = self.store.get_task(task_id).map_err(store_error)?;
        if task.is_none_or(|task| task.profile_id != profile_id) {
            return Err(BridgeError::new("task_not_found", "task is unavailable"));
        }
        Ok(())
    }

    fn remote_gateway_mut(&mut self) -> Result<&mut RemoteGateway, BridgeError> {
        self.remote_gateway.as_mut().ok_or_else(|| {
            BridgeError::new(
                "remote_gateway_unavailable",
                "remote gateway is unavailable",
            )
        })
    }

    fn active_remote_profile_id(&self) -> Option<String> {
        self.desktop_ui_state()
            .ok()
            .and_then(|state| state.active_profile_id)
    }

    fn require_active_remote_profile_id(&self) -> Result<String, BridgeError> {
        self.active_remote_profile_id().ok_or_else(|| {
            BridgeError::new(
                "profile_not_active",
                "select an account before managing remote devices",
            )
        })
    }

    fn remote_sidebar(&self, profile_id: &str) -> Result<Value, BridgeError> {
        let (profiles, mut projects, mut tasks, mut sections) =
            self.store.load_sidebar_records().map_err(store_error)?;
        let profiles = profiles
            .into_iter()
            .filter(|profile| profile.id == profile_id)
            .collect::<Vec<_>>();
        projects.retain(|project| project.profile_id.as_deref() == Some(profile_id));
        tasks.retain(|task| task.profile_id == profile_id);
        let project_ids = projects
            .iter()
            .map(|project| project.id.as_str())
            .collect::<HashSet<_>>();
        let task_ids = tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<HashSet<_>>();
        for section in &mut sections {
            section.items.retain(|item| match item {
                crate::permission_policy::SectionItem::Project(id) => {
                    project_ids.contains(id.as_str())
                }
                crate::permission_policy::SectionItem::Task(id) => task_ids.contains(id.as_str()),
            });
        }
        sections.retain(|section| !section.items.is_empty());
        let mut sidebar = crate::sidebar::CodexSidebarState {
            active_profile_id: Some(profile_id.to_string()),
            ..Default::default()
        };
        sidebar.replace_data(profiles, projects, tasks, sections);
        serde_json::to_value(crate::ui::codex_sidebar::snapshot(&sidebar))
            .map_err(|_| BridgeError::new("serialization_failed", "remote sidebar is unavailable"))
    }
}

fn validate_remote_fields(action: &str, object: &Map<String, Value>) -> Result<(), BridgeError> {
    let allowed: &[&str] = match action {
        "hello" | "bootstrap" | "list_sidebar" => &[],
        "history" | "start_task" | "retry_task" | "abort" | "stop" | "stop_task"
        | "runtime_snapshot" => &["task_id"],
        "prompt" => &["task_id", "prompt"],
        "create_task" => &["task_id", "project_id", "title", "summary"],
        "respond_ui" => &[
            "task_id",
            "request_id",
            "interaction_id",
            "response_kind",
            "value",
            "cancelled",
        ],
        "set_task" => &["task_id", "pinned", "archived", "unread"],
        _ => &[],
    };
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(BridgeError::new(
            "remote_field_denied",
            "one or more command fields are unavailable over PAD Remote",
        ));
    }
    Ok(())
}

fn required_string<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, BridgeError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| BridgeError::new("invalid_request", format!("missing {key}")))
}

fn optional_string(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn store_error(error: crate::pad_store::StoreError) -> BridgeError {
    BridgeError::from(DesktopRuntimeError::Store(error))
}

fn unavailable_status(error: Option<&str>) -> Value {
    json!({"remote": {
        "enabled": false,
        "state": "failed",
        "display_name": "PAD Desktop on Mac",
        "active_connections": 0,
        "devices": [],
        "updated_at": super::unix_timestamp(),
        "error_code": error.unwrap_or("remote_gateway_unavailable"),
    }})
}

fn safe_remote_error(code: &str) -> String {
    match code {
        "task_not_found" => "task is unavailable",
        "project_not_found" => "project is unavailable",
        "profile_unavailable" => "paired account is unavailable",
        "invalid_request" | "remote_field_denied" => "remote command is invalid",
        _ => "remote command failed",
    }
    .to_string()
}

fn remote_snapshot(snapshot: crate::pi_runtime::PiRuntimeSnapshot) -> Value {
    json!({
        "generation": snapshot.generation,
        "status": format!("{:?}", snapshot.status).to_ascii_lowercase(),
        "pending_message_count": snapshot.pending_message_count,
        "last_sequence": snapshot.last_sequence,
    })
}
