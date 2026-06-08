use crate::text_match::contains_ascii_ignore_case;
use serde_json::Value;

pub(super) fn extract_first_user_prompt(value: &Value) -> Option<String> {
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
    LOCAL_COMMAND_SCAFFOLD_TAGS
        .iter()
        .any(|tag| contains_ascii_ignore_case(text, tag))
}

const LOCAL_COMMAND_SCAFFOLD_TAGS: &[&str] = &[
    "<local-command-caveat>",
    "</local-command-caveat>",
    "<command-name>",
    "</command-name>",
    "<command-message>",
    "</command-message>",
    "<command-args>",
    "</command-args>",
];
