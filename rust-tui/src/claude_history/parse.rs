mod prompt;
mod time;

use super::model::IndexedClaudeThread;
use super::util::file_mtime_secs;
use prompt::extract_first_user_prompt;
use serde_json::Value;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use time::parse_rfc3339_utc_ts;

#[cfg(test)]
pub(crate) static PARSE_THREAD_FILE_CALLS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

pub(crate) fn parse_claude_thread_file(path: &Path) -> io::Result<Option<IndexedClaudeThread>> {
    #[cfg(test)]
    {
        PARSE_THREAD_FILE_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    if is_subagent_path(path) {
        return Ok(None);
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut session_id = None;
    let mut cwd = None;
    let mut title = None;
    let mut has_dialogue_event = false;
    let mut last_assistant_ts = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if is_sidechain_event(&value) {
            return Ok(None);
        }

        if session_id.is_none() {
            session_id = value
                .get("sessionId")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
        if cwd.is_none() {
            cwd = value.get("cwd").and_then(Value::as_str).map(PathBuf::from);
        }
        if title.is_none() {
            title = extract_first_user_prompt(&value);
        }
        if is_dialogue_event(&value) {
            has_dialogue_event = true;
        }
        if let Some(timestamp) = assistant_timestamp(&value) {
            last_assistant_ts = Some(last_assistant_ts.unwrap_or(timestamp).max(timestamp));
        }
    }

    let Some(session_id) = session_id else {
        return Ok(None);
    };
    let Some(cwd) = cwd else {
        return Ok(None);
    };
    if !has_dialogue_event {
        return Ok(None);
    }

    let updated_at = file_mtime_secs(path)?;
    let last_assistant_at = last_assistant_ts.unwrap_or(updated_at);

    Ok(Some(IndexedClaudeThread {
        session_id,
        cwd,
        transcript_path: path.to_path_buf(),
        title,
        updated_at,
        last_assistant_at,
    }))
}

fn is_subagent_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str().to_string_lossy() == "subagents")
}

fn is_sidechain_event(value: &Value) -> bool {
    value
        .get("isSidechain")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn is_dialogue_event(value: &Value) -> bool {
    matches!(
        value.get("type").and_then(Value::as_str),
        Some("user" | "assistant")
    )
}

fn assistant_timestamp(value: &Value) -> Option<i64> {
    if value.get("type").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc_ts)
}
