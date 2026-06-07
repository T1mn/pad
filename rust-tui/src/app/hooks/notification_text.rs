use crate::model::AgentType;
use std::path::Path;

pub(super) fn notification_agent_label(agent_type: &AgentType) -> &'static str {
    match agent_type {
        AgentType::Claude => "Claude",
        AgentType::Codex => "Codex",
        AgentType::Gemini => "Gemini",
        AgentType::OpenCode => "OpenCode",
        AgentType::Kimi => "Kimi",
        AgentType::Aider => "Aider",
        AgentType::Cursor => "Cursor",
        AgentType::Unknown => "Agent",
    }
}

pub(super) fn completion_notification_body(
    agent_type: &AgentType,
    session_id: Option<&str>,
    fallback_prompt: Option<&str>,
    working_dir: Option<&str>,
) -> String {
    fallback_prompt
        .map(normalize_notification_text)
        .or_else(|| lookup_notification_title(agent_type, session_id))
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| notification_workdir_fallback(working_dir, session_id))
}

fn lookup_notification_title(agent_type: &AgentType, session_id: Option<&str>) -> Option<String> {
    let session_id = session_id?;
    match agent_type {
        AgentType::Codex => crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title.or(thread.first_user_message))
            .map(normalize_notification_text),
        AgentType::Claude => crate::claude_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| thread.title)
            .map(normalize_notification_text),
        AgentType::Gemini => crate::gemini_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .and_then(|thread| {
                thread
                    .title
                    .or(thread.summary)
                    .or(thread.last_user_message)
                    .or(thread.first_user_message)
            })
            .map(normalize_notification_text),
        _ => None,
    }
}

fn normalize_notification_text(text: impl AsRef<str>) -> String {
    truncate_notification_text(
        &text
            .as_ref()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        72,
    )
}

fn truncate_notification_text(text: &str, max_chars: usize) -> String {
    let mut truncated = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars {
            truncated.push_str("...");
            return truncated;
        }
        truncated.push(ch);
    }
    truncated
}

fn notification_workdir_fallback(working_dir: Option<&str>, session_id: Option<&str>) -> String {
    working_dir
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .map(normalize_notification_text)
        .filter(|name| !name.is_empty())
        .or_else(|| session_id.map(normalize_notification_text))
        .unwrap_or_else(|| "Session complete".to_string())
}
