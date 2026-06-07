use serde_json::Value;

pub(super) fn extract_response_text(payload: &Value) -> Option<String> {
    if let Some(text) = payload.get("output_text").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    if let Some(text) = payload
        .pointer("/choices/0/message/content")
        .and_then(extract_content_text)
    {
        return Some(text);
    }

    payload
        .get("output")
        .and_then(Value::as_array)
        .and_then(|items| {
            let mut collected = Vec::new();
            for item in items {
                if let Some(content) = item.get("content").and_then(Value::as_array) {
                    for block in content {
                        if let Some(text) = block.get("text").and_then(Value::as_str) {
                            collected.push(text.trim());
                        }
                    }
                }
            }
            if collected.is_empty() {
                None
            } else {
                Some(collected.join("\n"))
            }
        })
}

pub(super) fn extract_error_text(payload: &Value) -> Option<String> {
    payload
        .pointer("/error/message")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            payload
                .get("message")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn extract_content_text(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if let Some(items) = value.as_array() {
        let mut collected = Vec::new();
        for item in items {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                collected.push(text.trim());
            }
        }
        if !collected.is_empty() {
            return Some(collected.join("\n"));
        }
    }
    None
}
