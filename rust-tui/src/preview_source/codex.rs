#[path = "codex/parser.rs"]
mod parser;
#[path = "codex/tail.rs"]
mod tail;

use super::SessionReadMode;
use crate::model::PreviewTurn;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const CODEX_STATUS_PROBE_CACHE_TTL: Duration = Duration::from_secs(6);
const CODEX_STATUS_PROBE_DEADLINE: Duration = Duration::from_millis(1200);
const CODEX_STATUS_PROBE_POLL_INTERVAL: Duration = Duration::from_millis(120);
const ENVIRONMENT_CONTEXT_OPEN: &str = "<environment_context>";
const ENVIRONMENT_CONTEXT_CLOSE: &str = "</environment_context>";
const TURN_ABORTED_OPEN: &str = "<turn_aborted>";
const TURN_ABORTED_CLOSE: &str = "</turn_aborted>";
const USER_SHELL_COMMAND_OPEN: &str = "<user_shell_command>";
const USER_SHELL_COMMAND_CLOSE: &str = "</user_shell_command>";
const SKILL_OPEN: &str = "<skill>";
const SKILL_CLOSE: &str = "</skill>";
const AGENTS_MD_INSTRUCTIONS_PREFIX: &str = "# AGENTS.md instructions for ";
const AGENTS_MD_INSTRUCTIONS_SUFFIX: &str = "</INSTRUCTIONS>";

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
    parser::parse_transcript(path, read_mode)
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

pub(crate) fn normalize_codex_user_text(text: &str, image_count_hint: Option<usize>) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Some(summary) = extract_user_shell_command_summary(trimmed) {
        return summary;
    }

    let stripped_context = strip_non_preview_codex_fragments(trimmed);
    let trimmed = stripped_context.trim();
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

fn strip_non_preview_codex_fragments(text: &str) -> String {
    let mut stripped = text.to_string();
    for (open, close) in [
        (ENVIRONMENT_CONTEXT_OPEN, ENVIRONMENT_CONTEXT_CLOSE),
        (TURN_ABORTED_OPEN, TURN_ABORTED_CLOSE),
        (USER_SHELL_COMMAND_OPEN, USER_SHELL_COMMAND_CLOSE),
        (SKILL_OPEN, SKILL_CLOSE),
        (AGENTS_MD_INSTRUCTIONS_PREFIX, AGENTS_MD_INSTRUCTIONS_SUFFIX),
    ] {
        stripped = strip_wrapped_block(&stripped, open, close);
    }
    stripped
}

fn strip_wrapped_block(text: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find(open) {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + open.len()..];
        let Some(end) = after_open.find(close) else {
            out.push_str(&rest[start..]);
            return out;
        };
        rest = &after_open[end + close.len()..];
    }

    out.push_str(rest);
    out
}

fn extract_user_shell_command_summary(text: &str) -> Option<String> {
    let inner = exact_wrapped_fragment(
        text.trim(),
        USER_SHELL_COMMAND_OPEN,
        USER_SHELL_COMMAND_CLOSE,
    )?;
    let command = find_wrapped_fragment(inner.trim(), "<command>", "</command>")
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(format!("[shell] {}", command))
}

fn exact_wrapped_fragment<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix(open)?.strip_suffix(close)?;
    Some(inner)
}

fn find_wrapped_fragment<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)?;
    let after_open = &text[start + open.len()..];
    let end = after_open.find(close)?;
    Some(&after_open[..end])
}

fn count_image_open_tags(text: &str) -> usize {
    text.match_indices("<image name=[Image #").count()
}

fn count_image_refs(text: &str) -> usize {
    let mut count = 0usize;
    let mut rest = text;
    while let Some(start) = rest.find("[Image #") {
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
    fn parse_codex_transcript_keeps_latest_real_user_turns() {
        let path = temp_jsonl_path("codex-history-limit");
        let mut content = String::new();
        for idx in 0..60 {
            content.push_str(&format!(
                "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"q{idx}\"}}]}}}}\n"
            ));
            content.push_str(&format!(
                "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{{\"type\":\"output_text\",\"text\":\"a{idx}\"}}]}}}}\n"
            ));
        }
        for _ in 0..20 {
            content.push_str(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<environment_context>\\n  <cwd>/tmp/demo</cwd>\\n</environment_context>\"}]}}\n",
            );
        }
        fs::write(&path, content).unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(
            turns.len(),
            crate::session_cache::SESSION_HISTORY_TURN_LIMIT
        );
        assert_eq!(turns[0].question, "q59");
        assert_eq!(turns[49].question, "q10");
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
    fn normalize_codex_user_text_filters_environment_context_block() {
        let text = "<environment_context>\n  <cwd>/tmp/demo</cwd>\n</environment_context>";
        assert_eq!(normalize_codex_user_text(text, None), "");
    }

    #[test]
    fn normalize_codex_user_text_strips_embedded_environment_context_block() {
        let text = "请分析一下\n<environment_context>\n  <cwd>/tmp/demo</cwd>\n</environment_context>\n这段结构";
        assert_eq!(
            normalize_codex_user_text(text, None),
            "请分析一下\n\n这段结构"
        );
    }

    #[test]
    fn normalize_codex_user_text_filters_turn_aborted_marker() {
        let text = "<turn_aborted>\ninterrupted\n</turn_aborted>";
        assert_eq!(normalize_codex_user_text(text, None), "");
    }

    #[test]
    fn normalize_codex_user_text_summarizes_user_shell_command() {
        let text = "<user_shell_command>\n<command>\necho hi\n</command>\n<result>\nExit code: 0\n</result>\n</user_shell_command>";
        assert_eq!(normalize_codex_user_text(text, None), "[shell] echo hi");
    }

    #[test]
    fn parse_codex_transcript_skips_context_only_user_messages() {
        let path = temp_jsonl_path("codex-context-filter");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<environment_context>\\n  <cwd>/tmp/demo</cwd>\\n</environment_context>\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"real question\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<turn_aborted>\\ninterrupted\\n</turn_aborted>\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"real answer\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "real question");
        assert_eq!(turns[0].answer.as_deref(), Some("real answer"));
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
