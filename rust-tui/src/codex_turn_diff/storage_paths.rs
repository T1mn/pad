use crate::hook::HookEvent;
use std::path::PathBuf;

const STORE_DIR: &str = "codex-turn-diffs";

pub fn storage_root() -> PathBuf {
    if let Some(path) = std::env::var_os("PAD_CODEX_TURN_DIFF_DIR") {
        return PathBuf::from(path);
    }
    crate::paths::pad_home_dir().join(STORE_DIR)
}

pub fn pending_dir() -> PathBuf {
    storage_root().join("pending")
}

pub fn records_dir() -> PathBuf {
    storage_root().join("records")
}

pub fn patches_dir() -> PathBuf {
    storage_root().join("patches")
}

pub fn index_path() -> PathBuf {
    storage_root().join("index.jsonl")
}

pub fn pending_path(id: &str) -> PathBuf {
    pending_dir().join(format!("{}.json", safe_name(id)))
}

pub fn record_path(id: &str) -> PathBuf {
    records_dir().join(format!("{}.json", safe_name(id)))
}

pub fn new_record_id(key: &str) -> String {
    format!("{}_{}", now_nanos(), safe_name(key))
}

pub fn event_key(event: &HookEvent) -> Option<String> {
    if let Some(turn_id) = event.turn_id.as_deref().filter(|value| !value.is_empty()) {
        return Some(format!("turn_{}", safe_name(turn_id)));
    }
    match (
        event
            .session_id
            .as_deref()
            .filter(|value| !value.is_empty()),
        event
            .tmux
            .pane_id
            .as_deref()
            .filter(|value| !value.is_empty()),
    ) {
        (Some(session_id), Some(pane_id)) => Some(format!(
            "session_{}_pane_{}",
            safe_name(session_id),
            safe_name(pane_id)
        )),
        (Some(session_id), None) => Some(format!("session_{}", safe_name(session_id))),
        (None, Some(pane_id)) => Some(format!("pane_{}", safe_name(pane_id))),
        (None, None) => None,
    }
}

pub fn now_stamp() -> String {
    crate::time::unix_now_secs().to_string()
}

fn safe_name(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').chars().take(96).collect()
}

fn now_nanos() -> u128 {
    crate::time::unix_now_nanos()
}
