use serde_json::Value;

pub(super) fn parse_message_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => clean_text(text),
        Value::Array(values) => parse_message_text_array(values),
        Value::Object(map) => parse_message_text_object(map),
        _ => None,
    }
}

fn parse_message_text_array(values: &[Value]) -> Option<String> {
    let mut joined = String::new();
    for part in values.iter().filter_map(parse_message_text) {
        if part.trim().is_empty() {
            continue;
        }
        if !joined.is_empty() {
            joined.push('\n');
        }
        joined.push_str(&part);
    }
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn parse_message_text_object(map: &serde_json::Map<String, Value>) -> Option<String> {
    ["text", "content", "parts"]
        .iter()
        .find_map(|key| map.get(*key).and_then(parse_message_text))
}

fn clean_text(text: &str) -> Option<String> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}
