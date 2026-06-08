use super::super::turns::SessionRole;
use serde_json::Value;

pub(super) fn message_text(value: &Value) -> Option<(SessionRole, String)> {
    if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
        return None;
    }

    let role = match value.get("type").and_then(Value::as_str)? {
        "user" => SessionRole::User,
        "assistant" => SessionRole::Assistant,
        _ => return None,
    };
    let message = value.get("message")?;
    let text = match role {
        SessionRole::User => extract_user_text(message),
        SessionRole::Assistant => extract_assistant_text(message),
    };

    Some((role, text))
}

fn extract_user_text(message: &Value) -> String {
    if message.get("role").and_then(Value::as_str) != Some("user") {
        return String::new();
    }

    match message.get("content") {
        Some(Value::String(text)) => sanitize_user_string(text),
        Some(Value::Array(items)) => join_non_empty_text(items.iter().filter_map(|item| {
            if item.get("type").and_then(Value::as_str) != Some("text") {
                return None;
            }
            item.get("text")
                .and_then(Value::as_str)
                .map(sanitize_user_string)
                .filter(|text| !text.is_empty())
        })),
        _ => String::new(),
    }
}

fn sanitize_user_string(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.contains("<command-name>")
        || trimmed.contains("<local-command")
    {
        return String::new();
    }

    trimmed.to_string()
}

fn extract_assistant_text(message: &Value) -> String {
    if message.get("role").and_then(Value::as_str) != Some("assistant") {
        return String::new();
    }

    match message.get("content") {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Array(items)) => join_non_empty_text(items.iter().filter_map(|item| {
            if item.get("type").and_then(Value::as_str) != Some("text") {
                return None;
            }
            item.get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
        })),
        _ => String::new(),
    }
}

fn join_non_empty_text<'a>(items: impl Iterator<Item = impl AsRef<str> + 'a>) -> String {
    let mut joined = String::new();
    for item in items {
        if !joined.is_empty() {
            joined.push('\n');
        }
        joined.push_str(item.as_ref());
    }
    joined
}
