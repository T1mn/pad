use crate::pi_runtime::{PiApprovalRequest, PiPoll, PiRuntimeSnapshot, PiRuntimeStatus};
use serde_json::{json, Value};

pub(super) fn poll_has_provider_auth_error(poll: &PiPoll) -> bool {
    poll.messages.iter().any(|message| {
        if message.message_type != "response"
            || message.value.get("success").and_then(Value::as_bool) != Some(false)
        {
            return false;
        }
        let text = serde_json::to_string(&message.value)
            .unwrap_or_default()
            .to_ascii_lowercase();
        [
            "no api key",
            "api key",
            "authentication",
            "unauthorized",
            "credential",
            "401",
            "please login",
        ]
        .iter()
        .any(|marker| text.contains(marker))
    })
}

pub(super) fn poll_value(poll: &PiPoll) -> Value {
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
        "pending_ui_requests": pending_ui_requests(poll),
        "stderr": String::from_utf8_lossy(&poll.stderr),
        "diagnostics": poll.diagnostics,
        "dropped_stale": poll.dropped_stale,
        "exit_status": exit_status,
    })
}

fn pending_ui_requests(poll: &PiPoll) -> Vec<Value> {
    poll.messages
        .iter()
        .filter(|message| message.message_type == "extension_ui_request")
        .filter_map(|message| {
            let request = PiApprovalRequest::parse(&message.value)?;
            let response_kind = match &request {
                PiApprovalRequest::Confirm { .. } => "confirm",
                PiApprovalRequest::Select { .. } => "select",
                PiApprovalRequest::Input { .. } => "input",
                PiApprovalRequest::Editor { .. } => "editor",
                PiApprovalRequest::Unknown { .. } => "unknown",
            };
            let id = request.id().map(str::to_string);
            let mut value = json!({
                "id": id,
                "kind": response_kind,
                "response_action": "respond_ui",
                "requires_response": !matches!(&request, PiApprovalRequest::Unknown { .. }),
            });
            let object = value.as_object_mut()?;
            match request {
                PiApprovalRequest::Confirm { title, message, .. } => {
                    object.insert("title".to_string(), title.into());
                    object.insert("message".to_string(), message.into());
                }
                PiApprovalRequest::Select {
                    title,
                    options,
                    default_index,
                    ..
                } => {
                    object.insert("title".to_string(), title.into());
                    object.insert("options".to_string(), json!(options));
                    object.insert("default_index".to_string(), default_index.into());
                }
                PiApprovalRequest::Input { title, default, .. }
                | PiApprovalRequest::Editor { title, default, .. } => {
                    object.insert("title".to_string(), title.into());
                    object.insert("default".to_string(), default.into());
                }
                PiApprovalRequest::Unknown { method, .. } => {
                    object.insert("method".to_string(), method.into());
                }
            }
            Some(value)
        })
        .collect()
}

pub(super) fn snapshot_value(snapshot: &PiRuntimeSnapshot) -> Value {
    json!({
        "generation": snapshot.generation,
        "status": runtime_status_name(snapshot.status),
        "pending_message_count": snapshot.pending_message_count,
        "active_tool_call_id": snapshot.active_tool_call_id,
        "last_sequence": snapshot.last_sequence,
    })
}

pub(super) fn runtime_status_name(status: PiRuntimeStatus) -> &'static str {
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
