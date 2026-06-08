use serde_json::Value;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

pub(in crate::chat) fn scan_codex_answer_updates(
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

fn codex_answer_line(line: &str, expected_turn_id: Option<&str>) -> Option<String> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    if let Some(answer) = codex_task_complete_line(&value, expected_turn_id) {
        return Some(answer);
    }
    codex_final_answer_line(&value)
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
    let mut answer = String::new();
    for item in payload.get("content")?.as_array()? {
        if item.get("type").and_then(Value::as_str) != Some("output_text") {
            continue;
        }
        let Some(text) = item
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
        else {
            continue;
        };
        if !answer.is_empty() {
            answer.push('\n');
        }
        answer.push_str(text);
    }
    if answer.is_empty() {
        return None;
    }
    Some(answer)
}

#[cfg(test)]
#[path = "answers_tests.rs"]
mod tests;
