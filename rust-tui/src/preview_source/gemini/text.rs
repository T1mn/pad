use serde_json::Value;

pub(super) fn extract_message_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.trim().to_string(),
        Value::Array(items) => extract_array_text(items),
        Value::Object(map) => extract_object_text(map),
        _ => String::new(),
    }
}

fn extract_array_text(items: &[Value]) -> String {
    let mut joined = String::new();
    for item in items {
        let text = extract_message_text(item);
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        if !joined.is_empty() {
            joined.push('\n');
        }
        joined.push_str(text);
    }
    joined
}

fn extract_object_text(map: &serde_json::Map<String, Value>) -> String {
    if let Some(text) = map.get("text").and_then(Value::as_str) {
        return text.trim().to_string();
    }
    if let Some(content) = map.get("content") {
        return extract_message_text(content);
    }
    if let Some(parts) = map.get("parts") {
        return extract_message_text(parts);
    }
    String::new()
}
