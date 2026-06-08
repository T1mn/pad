use super::{CodexFailureEvent, CodexFailureScanResult};
use serde_json::Value;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

pub(in crate::chat) fn scan_codex_failure_updates(
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
