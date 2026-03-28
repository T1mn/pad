use super::model::{GeminiSnapshot, GeminiThreadKey, GeminiThreadRecord};
use super::util::{file_mtime_secs, md5_hex, normalize_path, parse_timestamp, read_text};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn collect_records(root: &Path) -> io::Result<Vec<GeminiThreadRecord>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut groups: HashMap<GeminiThreadKey, Vec<GeminiSnapshot>> = HashMap::new();
    for session_path in walk_session_files(root)? {
        match parse_snapshot(&session_path) {
            Ok(Some(snapshot)) => {
                let key = GeminiThreadKey::new(
                    snapshot.session_id.clone(),
                    snapshot.project_root.to_string_lossy().to_string(),
                );
                groups.entry(key).or_default().push(snapshot);
            }
            Ok(None) => {}
            Err(_err) => {
                crate::log_debug!(
                    "gemini_history: skip unreadable snapshot path={} err={}",
                    session_path.display(),
                    _err
                );
            }
        }
    }

    let mut records = groups
        .into_iter()
        .filter_map(|(key, snapshots)| build_record(key, snapshots))
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.session_id.cmp(&left.session_id))
    });
    Ok(records)
}

fn build_record(
    _key: GeminiThreadKey,
    snapshots: Vec<GeminiSnapshot>,
) -> Option<GeminiThreadRecord> {
    let preferred = choose_preferred_snapshot(&snapshots)?;
    let updated_at = snapshots
        .iter()
        .map(|snapshot| snapshot.last_updated)
        .max()
        .unwrap_or(preferred.last_updated);
    let start_time = snapshots
        .iter()
        .map(|snapshot| snapshot.start_time)
        .min()
        .unwrap_or(preferred.start_time);
    let has_subagent = snapshots.iter().any(|snapshot| snapshot.kind == "subagent");
    let payload_hash = snapshots
        .iter()
        .map(|snapshot| snapshot.payload_hash.as_str())
        .collect::<Vec<_>>();
    let mut payload_hash = payload_hash;
    payload_hash.sort_unstable();
    let payload_hash = payload_hash.join(":");

    Some(GeminiThreadRecord {
        session_id: preferred.session_id.clone(),
        cwd: preferred.project_root.clone(),
        project_alias: preferred.project_alias.clone(),
        transcript_path: preferred.transcript_path.clone(),
        kind: preferred.kind.clone(),
        start_time,
        updated_at,
        title: preferred
            .summary
            .clone()
            .or_else(|| preferred.first_user_message.clone()),
        subtitle: preferred.last_user_message.clone(),
        summary: preferred.summary.clone(),
        first_user_message: preferred.first_user_message.clone(),
        last_user_message: preferred.last_user_message.clone(),
        last_assistant_message: preferred.last_assistant_message.clone(),
        has_subagent,
        payload_hash: md5_hex(&payload_hash),
        snapshot_count: snapshots.len() as i64,
    })
}

fn choose_preferred_snapshot(snapshots: &[GeminiSnapshot]) -> Option<&GeminiSnapshot> {
    snapshots
        .iter()
        .filter(|snapshot| snapshot.kind == "main")
        .max_by_key(|snapshot| (snapshot.last_updated, snapshot.start_time))
        .or_else(|| {
            snapshots
                .iter()
                .max_by_key(|snapshot| (snapshot.last_updated, snapshot.start_time))
        })
}

pub(crate) fn parse_snapshot(path: &Path) -> io::Result<Option<GeminiSnapshot>> {
    let text = read_text(path)?;
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(err) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse {}: {}", path.display(), err),
            ))
        }
    };

    let Some(session_id) = value.get("sessionId").and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some(kind) = value.get("kind").and_then(Value::as_str) else {
        return Ok(None);
    };

    let (project_root, project_alias) = project_root_for_session_path(path)?;
    let start_time = value
        .get("startTime")
        .and_then(Value::as_str)
        .and_then(parse_timestamp)
        .or_else(|| file_mtime_secs(path))
        .unwrap_or_default();
    let last_updated = value
        .get("lastUpdated")
        .and_then(Value::as_str)
        .and_then(parse_timestamp)
        .or_else(|| file_mtime_secs(path))
        .unwrap_or(start_time);
    let summary = value
        .get("summary")
        .and_then(parse_message_text)
        .or_else(|| {
            value
                .get("summary")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        });
    let transcript_path = path.to_path_buf();
    let mut first_user_message = None;
    let mut last_user_message = None;
    let mut last_assistant_message = None;

    if let Some(messages) = value.get("messages").and_then(Value::as_array) {
        for message in messages {
            let Some(message_type) = message.get("type").and_then(Value::as_str) else {
                continue;
            };
            let content = message.get("content").and_then(parse_message_text);
            if content.as_deref().map(str::trim).unwrap_or("").is_empty() {
                continue;
            }

            match message_type {
                "user" => {
                    if first_user_message.is_none() {
                        first_user_message = content.clone();
                    }
                    last_user_message = content;
                }
                "gemini" => {
                    last_assistant_message = content;
                }
                _ => {}
            }
        }
    }

    Ok(Some(GeminiSnapshot {
        session_id: session_id.to_string(),
        project_root,
        project_alias,
        transcript_path,
        kind: kind.to_string(),
        start_time,
        last_updated,
        summary,
        first_user_message,
        last_user_message,
        last_assistant_message,
        payload_hash: md5_hex(&text),
    }))
}

fn project_root_for_session_path(path: &Path) -> io::Result<(PathBuf, String)> {
    let project_dir = path
        .parent()
        .and_then(|parent| parent.parent())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid Gemini session path: {}", path.display()),
            )
        })?;
    let project_alias = project_dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("unknown"));
    let project_root_file = project_dir.join(".project_root");
    let project_root = fs::read_to_string(&project_root_file)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .map(PathBuf::from)
        .map(|path| normalize_path(&path))
        .unwrap_or_else(|| normalize_path(project_dir));
    Ok((project_root, project_alias))
}

fn walk_session_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if file_name.starts_with("session-") && file_name.ends_with(".json") {
                files.push(path);
            }
        }
    }

    Ok(files)
}

fn parse_message_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => clean_text(text),
        Value::Array(values) => {
            let parts = values
                .iter()
                .filter_map(parse_message_text)
                .filter(|part| !part.trim().is_empty())
                .collect::<Vec<_>>();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
            }
        }
        Value::Object(map) => {
            if let Some(text) = map.get("text") {
                return parse_message_text(text);
            }
            if let Some(content) = map.get("content") {
                return parse_message_text(content);
            }
            if let Some(parts) = map.get("parts") {
                return parse_message_text(parts);
            }
            None
        }
        _ => None,
    }
}

fn clean_text(text: &str) -> Option<String> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}
