use super::{DesktopRuntime, DesktopRuntimeError};
use crate::pi_runtime::{PiApprovalRequest, PiSupervisorError};
use serde_json::json;

impl DesktopRuntime {
    pub(crate) fn set_model(
        &self,
        task_id: &str,
        provider: &str,
        model_id: &str,
    ) -> Result<(), DesktopRuntimeError> {
        let active = self
            .active_tasks
            .get(task_id)
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        active.supervisor.send(json!({
            "type": "set_model",
            "provider": provider,
            "modelId": model_id,
        }))?;
        Ok(())
    }

    pub(crate) fn set_thinking_level(
        &self,
        task_id: &str,
        level: &str,
    ) -> Result<(), DesktopRuntimeError> {
        let active = self
            .active_tasks
            .get(task_id)
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        active.supervisor.send(json!({
            "type": "set_thinking_level",
            "level": level,
        }))?;
        Ok(())
    }

    pub(crate) fn remote_history_response(
        &mut self,
        task_id: &str,
    ) -> Result<Option<serde_json::Value>, DesktopRuntimeError> {
        if self.active_tasks.contains_key(task_id) {
            self.get_messages(task_id)
        } else {
            self.history(task_id).map(Some)
        }
    }

    pub(crate) fn respond_ui(
        &mut self,
        task_id: &str,
        request_id: &str,
        response_kind: Option<&str>,
        value: Option<serde_json::Value>,
        cancelled: bool,
    ) -> Result<(), DesktopRuntimeError> {
        if request_id.trim().is_empty() {
            return Err(DesktopRuntimeError::Pi(PiSupervisorError::InvalidCommand(
                "extension UI response id is empty".to_string(),
            )));
        }
        let active = self
            .active_tasks
            .get_mut(task_id)
            .ok_or_else(|| DesktopRuntimeError::TaskNotFound(task_id.to_string()))?;
        let request = active
            .pending_ui_requests
            .get(request_id)
            .and_then(|pending| PiApprovalRequest::parse(&pending.value))
            .ok_or_else(|| invalid_response("extension UI request is no longer pending"))?;
        let expected_kind = match &request {
            PiApprovalRequest::Confirm { .. } => "confirm",
            PiApprovalRequest::Select { .. } => "select",
            PiApprovalRequest::Input { .. } => "input",
            PiApprovalRequest::Editor { .. } => "editor",
            PiApprovalRequest::Unknown { .. } => {
                return Err(invalid_response("extension UI request cannot be answered"));
            }
        };
        if response_kind.is_some_and(|kind| kind != expected_kind) {
            return Err(invalid_response(
                "extension UI response kind does not match request",
            ));
        }
        let response = if cancelled {
            json!({
                "type": "extension_ui_response",
                "id": request_id,
                "cancelled": true,
            })
        } else {
            match request {
                PiApprovalRequest::Confirm { .. } => json!({
                    "type": "extension_ui_response",
                    "id": request_id,
                    "confirmed": value.as_ref().and_then(serde_json::Value::as_bool)
                        .ok_or_else(|| invalid_response("confirm response must be boolean"))?,
                }),
                PiApprovalRequest::Select { options, .. } => {
                    let selected = response_string(value.as_ref())?;
                    if !options.iter().any(|option| option == selected) {
                        return Err(invalid_response(
                            "select response is not one of the options",
                        ));
                    }
                    json!({"type":"extension_ui_response","id":request_id,"value":selected})
                }
                PiApprovalRequest::Input { .. } | PiApprovalRequest::Editor { .. } => {
                    json!({
                        "type":"extension_ui_response",
                        "id":request_id,
                        "value":response_string(value.as_ref())?,
                    })
                }
                PiApprovalRequest::Unknown { .. } => unreachable!(),
            }
        };
        active.supervisor.send(response)?;
        active.pending_ui_requests.remove(request_id);
        if active.pending_ui_requests.is_empty() {
            active.reducer.mark_ui_responded();
        }
        Ok(())
    }

    pub(crate) fn pending_ui_requests(&self, task_id: &str) -> Vec<serde_json::Value> {
        let now = std::time::Instant::now();
        self.active_tasks
            .get(task_id)
            .map(|active| {
                active
                    .pending_ui_requests
                    .values()
                    .filter(|pending| pending.expires_at.is_none_or(|deadline| deadline > now))
                    .filter_map(|pending| super::bridge::format::pending_ui_request(&pending.value))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn invalid_response(message: &str) -> DesktopRuntimeError {
    DesktopRuntimeError::Pi(PiSupervisorError::InvalidCommand(message.to_string()))
}

fn response_string(value: Option<&serde_json::Value>) -> Result<&str, DesktopRuntimeError> {
    value
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_response("extension UI response value must be a string"))
}
