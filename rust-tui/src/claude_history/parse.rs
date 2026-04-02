use super::model::IndexedClaudeThread;
use super::util::file_mtime_secs;
use serde_json::Value;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

#[cfg(test)]
pub(crate) static PARSE_THREAD_FILE_CALLS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

pub(crate) fn parse_claude_thread_file(path: &Path) -> io::Result<Option<IndexedClaudeThread>> {
    #[cfg(test)]
    {
        PARSE_THREAD_FILE_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    if path
        .components()
        .any(|component| component.as_os_str().to_string_lossy() == "subagents")
    {
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
        if value
            .get("isSidechain")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
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
        if matches!(
            value.get("type").and_then(Value::as_str),
            Some("user" | "assistant")
        ) {
            has_dialogue_event = true;
        }
        if value.get("type").and_then(Value::as_str) == Some("assistant") {
            if let Some(timestamp) = value
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(parse_rfc3339_utc_ts)
            {
                last_assistant_ts = Some(last_assistant_ts.unwrap_or(timestamp).max(timestamp));
            }
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

fn parse_rfc3339_utc_ts(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    let year = parse_i32(bytes, 0, 4)?;
    let month = parse_u32(bytes, 5, 7)?;
    let day = parse_u32(bytes, 8, 10)?;
    let hour = parse_u32(bytes, 11, 13)?;
    let minute = parse_u32(bytes, 14, 16)?;
    let second = parse_u32(bytes, 17, 19)?;
    let tz_start = text[19..]
        .find(['Z', '+', '-'])
        .map(|idx| idx + 19)
        .unwrap_or(bytes.len());
    let offset_secs = if bytes.get(tz_start) == Some(&b'Z') || tz_start == bytes.len() {
        0
    } else {
        let sign = if bytes.get(tz_start) == Some(&b'-') {
            -1_i64
        } else {
            1_i64
        };
        let offset_hour = parse_u32(bytes, tz_start + 1, tz_start + 3)? as i64;
        let offset_minute = parse_u32(bytes, tz_start + 4, tz_start + 6)? as i64;
        sign * (offset_hour * 3600 + offset_minute * 60)
    };

    let days = days_from_civil(year, month, day)?;
    Some(days * 86_400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64 - offset_secs)
}

fn parse_i32(bytes: &[u8], start: usize, end: usize) -> Option<i32> {
    std::str::from_utf8(bytes.get(start..end)?)
        .ok()?
        .parse()
        .ok()
}

fn parse_u32(bytes: &[u8], start: usize, end: usize) -> Option<u32> {
    std::str::from_utf8(bytes.get(start..end)?)
        .ok()?
        .parse()
        .ok()
}

fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let adjust = if month <= 2 { 1 } else { 0 };
    let year = year - adjust;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month as i32 + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era as i64 * 146_097 + doe as i64 - 719_468)
}

fn extract_first_user_prompt(value: &Value) -> Option<String> {
    if value.get("type").and_then(Value::as_str) != Some("user") {
        return None;
    }

    let message = value.get("message")?;
    if message.get("role").and_then(Value::as_str) != Some("user") {
        return None;
    }

    match message.get("content") {
        Some(Value::String(text)) => clean_text(text),
        Some(Value::Array(items)) => items.iter().find_map(|item| {
            if item.get("type").and_then(Value::as_str) != Some("text") {
                return None;
            }
            item.get("text")
                .and_then(Value::as_str)
                .and_then(clean_text)
        }),
        _ => None,
    }
}

fn clean_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || is_local_command_scaffold(trimmed) {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_local_command_scaffold(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("<local-command-caveat>")
        || lowered.contains("</local-command-caveat>")
        || lowered.contains("<command-name>")
        || lowered.contains("</command-name>")
        || lowered.contains("<command-message>")
        || lowered.contains("</command-message>")
        || lowered.contains("<command-args>")
        || lowered.contains("</command-args>")
}
