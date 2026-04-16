use serde_json::Value;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CodexApprovalRequest {
    pub(super) call_id: String,
    pub(super) justification: String,
}

pub(super) struct CodexApprovalScanResult {
    pub(super) active_request: Option<CodexApprovalRequest>,
    pub(super) next_offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CodexFailureEvent {
    pub(super) message: String,
    pub(super) error_info: Option<String>,
}

pub(super) struct CodexFailureScanResult {
    pub(super) failure: Option<CodexFailureEvent>,
    pub(super) next_offset: u64,
}

pub(super) fn scan_codex_answer_updates(
    path: &Path,
    offset: u64,
    turn_id: Option<&str>,
) -> io::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    let file = File::open(path)?;
    let len = file.metadata()?.len();
    let start = offset.min(len);
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(start))?;

    let mut latest_answer = None;
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        if let Some(answer) = codex_answer_line(line.trim(), turn_id) {
            latest_answer = Some(answer);
        }
        line.clear();
    }

    Ok(latest_answer)
}

pub(super) fn scan_codex_failure_updates(
    path: &Path,
    offset: u64,
    expected_turn_id: Option<&str>,
) -> io::Result<CodexFailureScanResult> {
    if !path.exists() {
        return Ok(CodexFailureScanResult {
            failure: None,
            next_offset: offset,
        });
    }

    let file = File::open(path)?;
    let len = file.metadata()?.len();
    let start = offset.min(len);
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(start))?;

    let mut failure = None;
    let mut next_offset = start;
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        next_offset += line.len() as u64;
        if let Some(event) = codex_failure_line(line.trim(), expected_turn_id) {
            failure = Some(event);
        }
        line.clear();
    }

    Ok(CodexFailureScanResult {
        failure,
        next_offset,
    })
}

pub(super) fn scan_codex_approval_updates(
    path: &Path,
    offset: u64,
    current_request: Option<CodexApprovalRequest>,
) -> io::Result<CodexApprovalScanResult> {
    if !path.exists() {
        return Ok(CodexApprovalScanResult {
            active_request: current_request,
            next_offset: offset,
        });
    }

    let file = File::open(path)?;
    let len = file.metadata()?.len();
    let start = offset.min(len);
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(start))?;

    let mut active_request = current_request;
    let mut next_offset = start;
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        next_offset += line.len() as u64;
        match codex_approval_line_update(line.trim()) {
            CodexApprovalLineUpdate::Request(request) => active_request = Some(request),
            CodexApprovalLineUpdate::Resolved(call_id) => {
                if active_request
                    .as_ref()
                    .map(|request| request.call_id.as_str())
                    == Some(call_id.as_str())
                {
                    active_request = None;
                }
            }
            CodexApprovalLineUpdate::None => {}
        }
        line.clear();
    }

    Ok(CodexApprovalScanResult {
        active_request,
        next_offset,
    })
}

enum CodexApprovalLineUpdate {
    None,
    Request(CodexApprovalRequest),
    Resolved(String),
}

fn codex_approval_line_update(line: &str) -> CodexApprovalLineUpdate {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return CodexApprovalLineUpdate::None;
    };
    if value.get("type").and_then(Value::as_str) != Some("response_item") {
        return CodexApprovalLineUpdate::None;
    }
    let Some(payload) = value.get("payload") else {
        return CodexApprovalLineUpdate::None;
    };
    match payload.get("type").and_then(Value::as_str) {
        Some("function_call") => extract_codex_approval_request(payload)
            .map(CodexApprovalLineUpdate::Request)
            .unwrap_or(CodexApprovalLineUpdate::None),
        Some("function_call_output") => payload
            .get("call_id")
            .and_then(Value::as_str)
            .map(|call_id| CodexApprovalLineUpdate::Resolved(call_id.to_string()))
            .unwrap_or(CodexApprovalLineUpdate::None),
        _ => CodexApprovalLineUpdate::None,
    }
}

fn extract_codex_approval_request(payload: &Value) -> Option<CodexApprovalRequest> {
    let call_id = payload.get("call_id").and_then(Value::as_str)?.trim();
    if call_id.is_empty() {
        return None;
    }

    let args_value = match payload.get("arguments") {
        Some(Value::String(raw)) => serde_json::from_str::<Value>(raw).ok()?,
        Some(value) => value.clone(),
        None => return None,
    };

    if args_value
        .get("sandbox_permissions")
        .and_then(Value::as_str)
        != Some("require_escalated")
    {
        return None;
    }
    let justification = args_value
        .get("justification")
        .and_then(Value::as_str)?
        .trim();
    if justification.is_empty() {
        return None;
    }

    Some(CodexApprovalRequest {
        call_id: call_id.to_string(),
        justification: justification.to_string(),
    })
}

fn codex_answer_line(line: &str, expected_turn_id: Option<&str>) -> Option<String> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    if let Some(answer) = codex_task_complete_line(&value, expected_turn_id) {
        return Some(answer);
    }
    codex_final_answer_line(&value)
}

fn codex_failure_line(line: &str, expected_turn_id: Option<&str>) -> Option<CodexFailureEvent> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    if value.get("type").and_then(Value::as_str) != Some("event_msg") {
        return None;
    }
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("error") {
        return None;
    }
    if let Some(expected_turn_id) = expected_turn_id {
        let actual_turn_id = payload
            .get("turn_id")
            .or_else(|| value.get("turn_id"))
            .and_then(Value::as_str);
        if let Some(actual_turn_id) = actual_turn_id {
            if actual_turn_id != expected_turn_id {
                return None;
            }
        }
    }

    let message = payload.get("message").and_then(Value::as_str)?.trim();
    if message.is_empty() {
        return None;
    }

    let error_info = payload
        .get("codex_error_info")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    Some(CodexFailureEvent {
        message: message.to_string(),
        error_info,
    })
}

fn codex_task_complete_line(value: &Value, expected_turn_id: Option<&str>) -> Option<String> {
    if value.get("type").and_then(Value::as_str) != Some("event_msg") {
        return None;
    }
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("task_complete") {
        return None;
    }
    if let Some(expected_turn_id) = expected_turn_id {
        let actual_turn_id = payload.get("turn_id").and_then(Value::as_str)?;
        if actual_turn_id != expected_turn_id {
            return None;
        }
    }
    payload
        .get("last_agent_message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

fn codex_final_answer_line(value: &Value) -> Option<String> {
    if value.get("type").and_then(Value::as_str) != Some("response_item") {
        return None;
    }
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }
    if payload.get("role").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    if payload.get("phase").and_then(Value::as_str) != Some("final_answer") {
        return None;
    }
    let content = payload.get("content")?.as_array()?;
    let parts = content
        .iter()
        .filter_map(|item| {
            if item.get("type").and_then(Value::as_str) != Some("output_text") {
                return None;
            }
            item.get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

pub(super) fn transcript_len(path: &str) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}
