pub(super) fn extract_response_text(payload: &serde_json::Value) -> Option<String> {
    if let Some(text) = payload.get("output_text").and_then(|value| value.as_str()) {
        return Some(text.to_string());
    }

    let mut out = String::new();
    let output = payload.get("output").and_then(|value| value.as_array())?;
    for item in output {
        let Some(content) = item.get("content").and_then(|value| value.as_array()) else {
            continue;
        };
        for content_item in content {
            if let Some(text) = content_item.get("text").and_then(|value| value.as_str()) {
                out.push_str(text);
            }
        }
    }
    Some(out)
}
