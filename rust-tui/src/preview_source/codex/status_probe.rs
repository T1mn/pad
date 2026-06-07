use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const CODEX_STATUS_PROBE_CACHE_TTL: Duration = Duration::from_secs(6);
const CODEX_STATUS_PROBE_DEADLINE: Duration = Duration::from_millis(1200);
const CODEX_STATUS_PROBE_POLL_INTERVAL: Duration = Duration::from_millis(120);

#[derive(Clone)]
struct StatusProbeCacheEntry {
    session_id: Option<String>,
    attempted_at: Instant,
}

static CODEX_STATUS_PROBE_CACHE: OnceLock<Mutex<HashMap<String, StatusProbeCacheEntry>>> =
    OnceLock::new();

pub(super) fn resolve_live_session_id(pane_id: &str) -> Option<String> {
    if let Some(cached) = cached_status_probe_result(pane_id) {
        return cached;
    }

    let session_id = probe_live_session_id(pane_id);
    remember_status_probe_result(pane_id, session_id.clone());
    session_id
}

fn cached_status_probe_result(pane_id: &str) -> Option<Option<String>> {
    let cache = CODEX_STATUS_PROBE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let guard = cache.lock().ok()?;
    let entry = guard.get(pane_id)?;
    if entry.attempted_at.elapsed() <= CODEX_STATUS_PROBE_CACHE_TTL {
        Some(entry.session_id.clone())
    } else {
        None
    }
}

fn remember_status_probe_result(pane_id: &str, session_id: Option<String>) {
    let cache = CODEX_STATUS_PROBE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = cache.lock() {
        guard.insert(
            pane_id.to_string(),
            StatusProbeCacheEntry {
                session_id,
                attempted_at: Instant::now(),
            },
        );
    }
}

fn probe_live_session_id(pane_id: &str) -> Option<String> {
    let baseline = crate::tmux_dispatch::capture_pane_tail(pane_id, 32).ok()?;
    let baseline = normalize_probe_capture(&baseline);
    crate::tmux_dispatch::dispatch_prompt(pane_id, "/status").ok()?;

    let started_at = Instant::now();
    while started_at.elapsed() < CODEX_STATUS_PROBE_DEADLINE {
        thread::sleep(CODEX_STATUS_PROBE_POLL_INTERVAL);
        let capture = crate::tmux_dispatch::capture_pane_tail(pane_id, 48).ok()?;
        let capture = normalize_probe_capture(&capture);
        if capture.is_empty() || capture == baseline {
            continue;
        }
        if let Some(session_id) = extract_status_session_id(&capture) {
            log_debug!(
                "preview.codex_probe: resolved pane={} session_id={}",
                pane_id,
                session_id
            );
            return Some(session_id);
        }
    }

    log_debug!(
        "preview.codex_probe: no session id found for pane={}",
        pane_id
    );
    None
}

fn normalize_probe_capture(capture: &str) -> String {
    capture
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub(super) fn extract_status_session_id(capture: &str) -> Option<String> {
    for token in capture.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '"' | '\'' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>'
            )
    }) {
        let trimmed = token.trim_matches(|ch: char| ch == ':' || ch == '.');
        if looks_like_uuid(trimmed) {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn looks_like_uuid(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return false;
    }

    for (idx, ch) in bytes.iter().enumerate() {
        let dash = matches!(idx, 8 | 13 | 18 | 23);
        if dash {
            if *ch != b'-' {
                return false;
            }
        } else if !(*ch as char).is_ascii_hexdigit() {
            return false;
        }
    }

    true
}
