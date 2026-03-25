use crate::i18n::{self, Locale};
use crate::model::{
    AgentPanel, AgentState, AgentType, PreviewSource, PreviewTurn, SessionCacheState,
};
use serde_json::Value;
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const TMUX_CAPTURE_LINES: usize = 50;
const BUSY_REFRESH_MS: u64 = 200;
const DEFAULT_REFRESH_MS: u64 = 500;

#[derive(Clone, Debug)]
pub struct PreviewUpdate {
    pub pane_id: String,
    pub content: String,
    pub source: PreviewSource,
    pub turns: Vec<PreviewTurn>,
    pub transcript_path: Option<String>,
    pub session_cache_state: Option<SessionCacheState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionRole {
    User,
    Assistant,
}

struct SessionPreviewData {
    turns: Vec<PreviewTurn>,
    transcript_path: Option<String>,
    cache_state: SessionCacheState,
}

#[derive(Clone, Copy)]
enum SessionReadMode {
    FullBackfill,
}

pub fn preview_refresh_interval_ms(panel: &AgentPanel) -> u64 {
    if matches!(panel.state, AgentState::Busy) {
        BUSY_REFRESH_MS
    } else {
        DEFAULT_REFRESH_MS
    }
}

pub fn load_preview(panel: &AgentPanel, mode: &str, locale: Locale) -> PreviewUpdate {
    let preferred_source = resolve_preferred_source(panel, mode);
    let (content, source, turns, transcript_path, session_cache_state) = match preferred_source {
        PreviewSource::Tmux => (
            load_tmux_preview(panel),
            PreviewSource::Tmux,
            Vec::new(),
            None,
            None,
        ),
        PreviewSource::Session => match load_session_preview(panel, locale) {
            Ok(data) => (
                format_session_turns(&data.turns),
                PreviewSource::Session,
                data.turns,
                data.transcript_path,
                Some(data.cache_state),
            ),
            Err(_err) if mode == "auto" => (
                load_tmux_preview(panel),
                PreviewSource::Tmux,
                Vec::new(),
                None,
                None,
            ),
            Err(err) => (err, PreviewSource::Session, Vec::new(), None, None),
        },
    };

    PreviewUpdate {
        pane_id: panel.pane_id.clone(),
        content,
        source,
        turns,
        transcript_path,
        session_cache_state,
    }
}

fn resolve_preferred_source(panel: &AgentPanel, mode: &str) -> PreviewSource {
    match mode {
        "tmux" => PreviewSource::Tmux,
        "session" => PreviewSource::Session,
        _ => {
            if supports_session_preview(panel) {
                PreviewSource::Session
            } else {
                PreviewSource::Tmux
            }
        }
    }
}

fn supports_session_preview(panel: &AgentPanel) -> bool {
    matches!(panel.agent_type, AgentType::Claude | AgentType::Codex)
        && (panel.transcript_path.is_some()
            || panel.agent_session_id.is_some()
            || !panel.cached_preview_turns.is_empty())
}

fn load_tmux_preview(panel: &AgentPanel) -> String {
    match crate::pty::capture_pane(&panel.pane_id, TMUX_CAPTURE_LINES) {
        Ok(content) => content,
        Err(_) => String::from("Failed to capture pane"),
    }
}

fn load_session_preview(panel: &AgentPanel, locale: Locale) -> Result<SessionPreviewData, String> {
    let needs_backfill = panel.cached_preview_turns.len()
        < crate::session_cache::SESSION_HISTORY_TURN_LIMIT
        || panel.session_cache_state != Some(SessionCacheState::Confirmed);

    if !needs_backfill && !panel.cached_preview_turns.is_empty() {
        return Ok(cached_session_preview(panel));
    }

    if let Some(transcript_path) = resolve_transcript_path(panel) {
        let turns = match panel.agent_type {
            AgentType::Codex => {
                parse_codex_transcript(&transcript_path, SessionReadMode::FullBackfill)
            }
            AgentType::Claude => {
                parse_claude_transcript(&transcript_path, SessionReadMode::FullBackfill)
            }
            _ => Ok(Vec::new()),
        };

        match turns {
            Ok(turns) if !turns.is_empty() => {
                let transcript_string = transcript_path.to_string_lossy().to_string();
                if let Err(err) =
                    crate::session_cache::persist_resolved_session(panel, &transcript_path, &turns)
                {
                    log_debug!("session_cache: persist resolved failed: {}", err);
                }
                return Ok(SessionPreviewData {
                    turns,
                    transcript_path: Some(transcript_string),
                    cache_state: SessionCacheState::Confirmed,
                });
            }
            Ok(_) => {
                if !panel.cached_preview_turns.is_empty() {
                    return Ok(cached_session_preview(panel));
                }
                return Err(session_unavailable_message(
                    locale,
                    i18n::t(locale, "preview.session_empty"),
                ));
            }
            Err(err) => {
                if !panel.cached_preview_turns.is_empty() {
                    return Ok(cached_session_preview(panel));
                }
                return Err(session_unavailable_message(
                    locale,
                    &format!(
                        "{}: {}",
                        i18n::t(locale, "preview.session_parse_failed"),
                        err
                    ),
                ));
            }
        }
    }

    if !panel.cached_preview_turns.is_empty() {
        return Ok(cached_session_preview(panel));
    }

    Err(session_unavailable_message(
        locale,
        i18n::t(locale, "preview.session_missing"),
    ))
}

fn cached_session_preview(panel: &AgentPanel) -> SessionPreviewData {
    SessionPreviewData {
        turns: panel.cached_preview_turns.clone(),
        transcript_path: panel.transcript_path.clone(),
        cache_state: panel
            .session_cache_state
            .unwrap_or(SessionCacheState::Cached),
    }
}

fn session_unavailable_message(locale: Locale, detail: &str) -> String {
    format!(
        "{}\n\n{}",
        i18n::t(locale, "preview.session_unavailable"),
        detail
    )
}

fn resolve_transcript_path(panel: &AgentPanel) -> Option<PathBuf> {
    if let Some(path) = panel.transcript_path.as_ref() {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let session_id = panel.agent_session_id.as_deref()?;
    match panel.agent_type {
        AgentType::Codex => {
            find_matching_jsonl(&dirs::home_dir()?.join(".codex").join("sessions"), |name| {
                name.ends_with(".jsonl") && name.contains(session_id)
            })
        }
        AgentType::Claude => {
            let expected = format!("{}.jsonl", session_id);
            find_matching_jsonl(
                &dirs::home_dir()?.join(".claude").join("projects"),
                |name| name == expected,
            )
        }
        _ => None,
    }
}

fn find_matching_jsonl<F>(root: &Path, matcher: F) -> Option<PathBuf>
where
    F: Fn(&str) -> bool,
{
    if !root.exists() {
        return None;
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let file_name = path.file_name()?.to_string_lossy();
            if matcher(&file_name) {
                return Some(path);
            }
        }
    }

    None
}

fn parse_codex_transcript(
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

        if payload.get("type").and_then(Value::as_str) != Some("message") {
            return;
        }

        let role = match payload.get("role").and_then(Value::as_str) {
            Some("user") => SessionRole::User,
            Some("assistant") => SessionRole::Assistant,
            _ => return,
        };

        let text = extract_codex_message_text(payload, role);
        push_session_message(&mut turns, role, text);
    })
    .map_err(|err| err.to_string())?;

    Ok(finalize_turns(turns))
}

fn extract_codex_message_text(payload: &Value, role: SessionRole) -> String {
    let Some(content) = payload.get("content").and_then(Value::as_array) else {
        return String::new();
    };

    let target_type = match role {
        SessionRole::User => "input_text",
        SessionRole::Assistant => "output_text",
    };

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

fn parse_claude_transcript(
    path: &Path,
    read_mode: SessionReadMode,
) -> Result<Vec<PreviewTurn>, String> {
    let mut turns = VecDeque::new();

    for_each_session_line(path, read_mode, |line| {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return;
        };

        if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
            return;
        }

        let role = match value.get("type").and_then(Value::as_str) {
            Some("user") => SessionRole::User,
            Some("assistant") => SessionRole::Assistant,
            _ => return,
        };

        let Some(message) = value.get("message") else {
            return;
        };

        let text = match role {
            SessionRole::User => extract_claude_user_text(message),
            SessionRole::Assistant => extract_claude_assistant_text(message),
        };

        push_session_message(&mut turns, role, text);
    })
    .map_err(|err| err.to_string())?;

    Ok(finalize_turns(turns))
}

fn extract_claude_user_text(message: &Value) -> String {
    if message.get("role").and_then(Value::as_str) != Some("user") {
        return String::new();
    }

    match message.get("content") {
        Some(Value::String(text)) => sanitize_claude_user_string(text),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                if item.get("type").and_then(Value::as_str) != Some("text") {
                    return None;
                }
                item.get("text")
                    .and_then(Value::as_str)
                    .map(sanitize_claude_user_string)
                    .filter(|text| !text.is_empty())
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn sanitize_claude_user_string(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.contains("<command-name>")
        || trimmed.contains("<local-command")
    {
        return String::new();
    }

    trimmed.to_string()
}

fn extract_claude_assistant_text(message: &Value) -> String {
    if message.get("role").and_then(Value::as_str) != Some("assistant") {
        return String::new();
    }

    match message.get("content") {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                if item.get("type").and_then(Value::as_str) != Some("text") {
                    return None;
                }
                item.get("text")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
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

fn push_session_message(turns: &mut VecDeque<PreviewTurn>, role: SessionRole, text: String) {
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }

    match role {
        SessionRole::User => {
            turns.push_back(PreviewTurn {
                question: text,
                answer: None,
            });
            while turns.len() > crate::session_cache::SESSION_HISTORY_TURN_LIMIT {
                turns.pop_front();
            }
        }
        SessionRole::Assistant => {
            if let Some(last) = turns.back_mut() {
                match last.answer.as_mut() {
                    Some(existing) => {
                        if !existing.is_empty() {
                            existing.push('\n');
                        }
                        existing.push_str(&text);
                    }
                    None => {
                        last.answer = Some(text);
                    }
                }
            }
        }
    }
}

fn finalize_turns(turns: VecDeque<PreviewTurn>) -> Vec<PreviewTurn> {
    turns.into_iter().rev().collect()
}

fn format_session_turns(turns: &[PreviewTurn]) -> String {
    turns
        .iter()
        .map(|turn| {
            let answer = turn.answer.as_deref().unwrap_or("...");
            format!("Q:\n{}\n\nA:\n{}", turn.question.trim(), answer.trim())
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::{parse_claude_transcript, parse_codex_transcript, SessionReadMode};
    use std::fs;
    use std::path::PathBuf;
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

        let turns = parse_codex_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "hello");
        assert_eq!(turns[0].answer.as_deref(), Some("world"));
    }

    #[test]
    fn parse_claude_transcript_skips_meta_thinking_and_tools() {
        let path = temp_jsonl_path("claude");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"user\",\"isMeta\":true,\"message\":{\"role\":\"user\",\"content\":\"skip meta\"}}\n",
                "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"<command-name>/clear</command-name>\"}}\n",
                "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"real user\"}}\n",
                "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"skip\"},{\"type\":\"text\",\"text\":\"real assistant\"}]}}\n",
                "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"tool_result\",\"content\":\"skip tool\"}]}}\n"
            ),
        )
        .unwrap();

        let turns = parse_claude_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "real user");
        assert_eq!(turns[0].answer.as_deref(), Some("real assistant"));
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

        let turns = parse_codex_transcript(&path, SessionReadMode::FullBackfill).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(turns.len(), 8);
        assert_eq!(turns[0].question, "q7");
        assert_eq!(turns[7].question, "q0");
    }
}
