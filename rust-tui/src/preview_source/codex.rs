use super::turns::{finalize_turns, push_session_message, SessionRole};
use super::SessionReadMode;
use crate::model::PreviewTurn;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
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

pub(super) fn parse_transcript(
    path: &Path,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    let mut turns = VecDeque::new();

    for_each_session_line(path, read_mode, |line| {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return;
        };

        if value.get("type").and_then(Value::as_str) != Some("response_item") {
            return;
        }

        let payload = match value.get("payload") {
            Some(payload) => payload,
            None => return,
        };

        match payload.get("type").and_then(Value::as_str) {
            Some("message") => {
                let role = match payload.get("role").and_then(Value::as_str) {
                    Some("user") => SessionRole::User,
                    Some("assistant") => SessionRole::Assistant,
                    _ => return,
                };

                let (effective_role, text) = extract_message_text(payload, role);
                push_session_message(&mut turns, effective_role, text);
            }
            Some("function_call") => {
                if let Some(summary) = extract_spawn_agent_event_text(payload) {
                    push_session_message(&mut turns, SessionRole::Assistant, summary);
                }
            }
            _ => {}
        }
    })
    .map_err(|err| err.to_string())?;

    Ok(finalize_turns(turns))
}

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

fn extract_message_text(payload: &Value, role: SessionRole) -> (SessionRole, String) {
    let Some(content) = payload.get("content").and_then(Value::as_array) else {
        return (role, String::new());
    };

    if role == SessionRole::User {
        let text = extract_codex_user_message_text(content);
        if let Some(summary) = extract_subagent_notification_summary(&text) {
            return (SessionRole::Assistant, summary);
        }
        return (role, text);
    }

    (role, join_message_text(content, "output_text"))
}

fn join_message_text(content: &[Value], target_type: &str) -> String {
    content
        .iter()
        .filter_map(|item| {
            if item.get("type").and_then(Value::as_str) != Some(target_type) {
                return None;
            }
            item.get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_codex_user_message_text(content: &[Value]) -> String {
    let mut image_count = 0usize;
    let mut parts = Vec::new();

    for item in content {
        match item.get("type").and_then(Value::as_str) {
            Some("input_image") => image_count += 1,
            Some("input_text") => {
                if let Some(text) = item
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                {
                    parts.push(text);
                }
            }
            _ => {}
        }
    }

    normalize_codex_user_text(&parts.join("\n"), Some(image_count))
}

pub(crate) fn normalize_codex_user_text(text: &str, image_count_hint: Option<usize>) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let contains_image_wrappers =
        trimmed.contains("<image name=[Image #") || trimmed.contains("</image>");
    let starts_with_image_ref = trimmed.starts_with("[Image #");
    let image_count = image_count_hint
        .filter(|count| *count > 0)
        .or_else(|| {
            let open_tag_count = count_image_open_tags(trimmed);
            (open_tag_count > 0).then_some(open_tag_count)
        })
        .or_else(|| starts_with_image_ref.then(|| count_image_refs(trimmed)))
        .unwrap_or(0);

    if image_count == 0 && !contains_image_wrappers && !starts_with_image_ref {
        return trimmed.to_string();
    }

    let without_wrappers = trimmed
        .lines()
        .filter(|line| !is_image_wrapper_line(line.trim()))
        .collect::<Vec<_>>()
        .join("\n");
    let stripped = strip_all_image_refs(&without_wrappers);
    let body = stripped
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if image_count == 0 {
        return trimmed.to_string();
    }

    if body.is_empty() {
        format!("[Image x{}]", image_count)
    } else {
        format!("[Image x{}] {}", image_count, body)
    }
}

fn count_image_open_tags(text: &str) -> usize {
    text.match_indices("<image name=[Image #").count()
}

fn count_image_refs(text: &str) -> usize {
    let mut count = 0usize;
    let mut rest = text;
    loop {
        let Some(start) = rest.find("[Image #") else {
            break;
        };
        let candidate = &rest[start..];
        let Some(end) = candidate.find(']') else {
            break;
        };
        if is_image_ref_token(&candidate[..=end]) {
            count += 1;
            rest = &candidate[end + 1..];
        } else {
            rest = &candidate["[Image #".len()..];
        }
    }
    count
}

fn strip_all_image_refs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("[Image #") {
        out.push_str(&rest[..start]);
        let candidate = &rest[start..];
        let Some(end) = candidate.find(']') else {
            out.push_str(candidate);
            return out;
        };
        let token = &candidate[..=end];
        if is_image_ref_token(token) {
            rest = &candidate[end + 1..];
        } else {
            out.push_str("[Image #");
            rest = &candidate["[Image #".len()..];
        }
    }

    out.push_str(rest);
    out
}

fn is_image_wrapper_line(line: &str) -> bool {
    line == "</image>" || is_image_open_tag(line)
}

fn is_image_open_tag(text: &str) -> bool {
    let Some(inner) = text
        .strip_prefix("<image name=[Image #")
        .and_then(|value| value.strip_suffix("]>"))
    else {
        return false;
    };

    !inner.is_empty() && inner.chars().all(|ch| ch.is_ascii_digit())
}

fn is_image_ref_token(text: &str) -> bool {
    let Some(inner) = text
        .strip_prefix("[Image #")
        .and_then(|value| value.strip_suffix(']'))
    else {
        return false;
    };

    !inner.is_empty() && inner.chars().all(|ch| ch.is_ascii_digit())
}

fn extract_spawn_agent_event_text(payload: &Value) -> Option<String> {
    if payload.get("name").and_then(Value::as_str) != Some("spawn_agent") {
        return None;
    }

    let arguments = payload
        .get("arguments")
        .and_then(Value::as_str)
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

fn extract_subagent_notification_summary(text: &str) -> Option<String> {
    const OPEN: &str = "<subagent_notification>";
    const CLOSE: &str = "</subagent_notification>";

    let start = text.find(OPEN)?;
    let rest = &text[start + OPEN.len()..];
    let end = rest.find(CLOSE)?;
    let json = rest[..end].trim();
    let value = serde_json::from_str::<Value>(json).ok()?;

    let agent_path = value
        .get("agent_path")
        .and_then(Value::as_str)
        .unwrap_or("subagent");
    let agent_label = agent_path.rsplit('/').next().unwrap_or(agent_path);
    let status = value.get("status").and_then(Value::as_object);
    let (status_label, detail) = if let Some(status) = status {
        if let Some(completed) = status.get("completed").and_then(Value::as_str) {
            ("completed", completed)
        } else if let Some(failed) = status.get("failed").and_then(Value::as_str) {
            ("failed", failed)
        } else if let Some(running) = status.get("running").and_then(Value::as_str) {
            ("running", running)
        } else {
            ("updated", "")
        }
    } else {
        ("updated", "")
    };

    let compact = compact_subagent_detail(detail);
    if compact.is_empty() {
        Some(format!("[subagent/{}] {}", status_label, agent_label))
    } else {
        Some(format!(
            "[subagent/{}] {}\n{}",
            status_label, agent_label, compact
        ))
    }
}

fn compact_subagent_detail(text: &str) -> String {
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    if line.is_empty() {
        return String::new();
    }

    let compact = line.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars_with_ellipsis(&compact, 220)
}

fn truncate_chars_with_ellipsis(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut out = String::new();
    for ch in text.chars().take(max_chars.saturating_sub(1)) {
        out.push(ch);
    }
    out.push('…');
    out
}

fn for_each_session_line<F>(
    path: &Path,
    read_mode: SessionReadMode,
    mut f: F,
) -> std::io::Result<()>
where
    F: FnMut(&str),
{
    match read_mode {
        SessionReadMode::FullBackfill => {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                f(&line?);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{extract_status_session_id, normalize_codex_user_text, parse_transcript};
    use crate::preview_source::SessionReadMode;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::Instant;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_jsonl_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pad-preview-{}-{}.jsonl", name, stamp))
    }

    #[test]
    fn parse_codex_transcript_extracts_recent_messages() {
        let path = temp_jsonl_path("codex");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"developer\",\"content\":[{\"type\":\"input_text\",\"text\":\"skip\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hello\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"world\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "hello");
        assert_eq!(turns[0].answer.as_deref(), Some("world"));
    }

    #[test]
    fn parse_codex_transcript_backfills_beyond_six_turns() {
        let path = temp_jsonl_path("codex-history");
        let mut content = String::new();
        for idx in 0..8 {
            content.push_str(&format!(
                "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"q{idx}\"}}]}}}}\n"
            ));
            content.push_str(&format!(
                "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{{\"type\":\"output_text\",\"text\":\"a{idx}\"}}]}}}}\n"
            ));
        }
        fs::write(&path, content).unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 8);
        assert_eq!(turns[0].question, "q7");
        assert_eq!(turns[7].question, "q0");
    }

    #[test]
    fn parse_codex_transcript_includes_subagent_events_in_main_turn() {
        let path = temp_jsonl_path("codex-subagent");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"run 2 subagents\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"spawn_agent\",\"arguments\":\"{\\\"agent_type\\\":\\\"explorer\\\",\\\"task_name\\\":\\\"audit_event_rs\\\"}\"}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<subagent_notification>\\n{\\\"agent_path\\\":\\\"/root/audit_event_rs\\\",\\\"status\\\":{\\\"completed\\\":\\\"`src/event.rs` is structurally overloaded and should be split into layers.\\\"}}\\n</subagent_notification>\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"merged result\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        let answer = turns[0].answer.as_deref().unwrap_or("");
        assert_eq!(turns[0].question, "run 2 subagents");
        assert!(answer.contains("[subagent/start][explorer] audit_event_rs"));
        assert!(answer.contains("[subagent/completed] audit_event_rs"));
        assert!(answer.contains("merged result"));
        assert!(!answer.contains("<subagent_notification>"));
    }

    #[test]
    fn parse_codex_transcript_normalizes_single_image_user_message() {
        let path = temp_jsonl_path("codex-image-single");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[",
                "{\"type\":\"input_text\",\"text\":\"<image name=[Image #1]>\"},",
                "{\"type\":\"input_image\",\"image_url\":\"file:///tmp/1.png\"},",
                "{\"type\":\"input_text\",\"text\":\"</image>\"},",
                "{\"type\":\"input_text\",\"text\":\"[Image #1] 为什么 settings 底部有黑边？\"}",
                "]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"看一下\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns[0].question, "[Image x1] 为什么 settings 底部有黑边？");
    }

    #[test]
    fn parse_codex_transcript_normalizes_multiple_image_user_message() {
        let path = temp_jsonl_path("codex-image-multi");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[",
                "{\"type\":\"input_text\",\"text\":\"<image name=[Image #1]>\"},",
                "{\"type\":\"input_image\",\"image_url\":\"file:///tmp/1.png\"},",
                "{\"type\":\"input_text\",\"text\":\"</image>\"},",
                "{\"type\":\"input_text\",\"text\":\"<image name=[Image #2]>\"},",
                "{\"type\":\"input_image\",\"image_url\":\"file:///tmp/2.png\"},",
                "{\"type\":\"input_text\",\"text\":\"</image>\"},",
                "{\"type\":\"input_text\",\"text\":\"[Image #1] 左侧不对，[Image #2] 右侧也不对\"}",
                "]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns[0].question, "[Image x2] 左侧不对， 右侧也不对");
    }

    #[test]
    fn normalize_codex_user_text_handles_image_only_message() {
        let text = "<image name=[Image #1]>\n</image>\n[Image #1]";
        assert_eq!(normalize_codex_user_text(text, Some(1)), "[Image x1]");
    }

    #[test]
    fn normalize_codex_user_text_does_not_touch_plain_text_without_images() {
        let text = "literal [Image #1] text";
        assert_eq!(normalize_codex_user_text(text, None), text);
    }

    #[test]
    fn status_probe_extracts_uuid_like_session_id() {
        let capture = "Codex status\nSession ID: 123e4567-e89b-12d3-a456-426614174000\nIdle";
        assert_eq!(
            extract_status_session_id(capture),
            Some("123e4567-e89b-12d3-a456-426614174000".to_string())
        );
    }

    #[test]
    #[ignore]
    fn bench_parse_transcripts_from_env() {
        let raw_paths = std::env::var("PAD_CODEX_BENCH_PATHS")
            .expect("set PAD_CODEX_BENCH_PATHS to a ';'-separated list of transcript paths");
        let iterations = std::env::var("PAD_CODEX_BENCH_ITERATIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5);

        for raw_path in raw_paths
            .split(';')
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            let path = Path::new(raw_path);
            let metadata = fs::metadata(path)
                .unwrap_or_else(|err| panic!("failed to stat {}: {}", path.display(), err));
            let mut elapsed_ms = Vec::with_capacity(iterations);
            let mut turn_count = None;

            for _ in 0..iterations {
                let started_at = Instant::now();
                let turns = parse_transcript(path, SessionReadMode::FullBackfill)
                    .unwrap_or_else(|err| panic!("failed to parse {}: {}", path.display(), err));
                elapsed_ms.push(started_at.elapsed().as_secs_f64() * 1000.0);
                turn_count = Some(turns.len());
            }

            let total_ms: f64 = elapsed_ms.iter().sum();
            let avg_ms = total_ms / elapsed_ms.len() as f64;
            let min_ms = elapsed_ms.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_ms = elapsed_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            println!(
                "bench.codex_parse path={} bytes={} turns={} iterations={} runs_ms={:?} avg_ms={:.3} min_ms={:.3} max_ms={:.3}",
                path.display(),
                metadata.len(),
                turn_count.unwrap_or(0),
                iterations,
                elapsed_ms,
                avg_ms,
                min_ms,
                max_ms
            );
        }
    }
}
