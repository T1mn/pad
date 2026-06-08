use super::model::TranscriptPayload;
use serde_json::Value;

pub(super) fn extract_spawn_agent_event_text_from_payload(
    payload: &TranscriptPayload<'_>,
) -> Option<String> {
    if payload.name.as_deref() != Some("spawn_agent") {
        return None;
    }

    let arguments = payload
        .arguments
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok());

    let task_name = arguments
        .as_ref()
        .and_then(|value| value.get("task_name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let agent_type = arguments
        .as_ref()
        .and_then(|value| value.get("agent_type"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let kind = agent_type.unwrap_or("worker");
    let task = task_name.unwrap_or("task");
    Some(format!("[subagent/start][{}] {}", kind, task))
}
