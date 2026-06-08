use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OpenCodeRole {
    User,
    Assistant,
}

pub(crate) fn message_role(raw: &str) -> Option<OpenCodeRole> {
    let value = serde_json::from_str::<Value>(raw).ok()?;
    match value.get("role").and_then(Value::as_str)? {
        "user" => Some(OpenCodeRole::User),
        "assistant" => Some(OpenCodeRole::Assistant),
        _ => None,
    }
}

pub(crate) fn extract_any_part_text(raw: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(raw).ok()?;
    extract_text_value(&value)
}

pub(crate) fn extract_display_part_text(raw: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(raw).ok()?;
    match value.get("type").and_then(Value::as_str) {
        Some("text") | Some("reasoning") | Some("step-start") | None => extract_text_value(&value),
        _ => None,
    }
}

fn extract_text_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => non_empty(text),
        Value::Array(items) => join_text(items.iter().filter_map(extract_text_value)),
        Value::Object(map) => {
            for key in ["text", "content", "message", "value"] {
                if let Some(text) = map.get(key).and_then(extract_text_value) {
                    return Some(text);
                }
            }
            None
        }
        _ => None,
    }
}

fn join_text(items: impl Iterator<Item = String>) -> Option<String> {
    let mut text = String::new();
    for item in items {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&item);
    }
    non_empty(&text)
}

fn non_empty(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
#[path = "opencode_text_tests.rs"]
mod tests;
