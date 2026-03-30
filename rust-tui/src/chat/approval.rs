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

pub(super) fn transcript_len(path: &str) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}
