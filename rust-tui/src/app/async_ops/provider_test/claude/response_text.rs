pub(super) fn extract_response_text(payload: &serde_json::Value) -> Option<String> {
    let mut out = String::new();
    let content = payload.get("content")?.as_array()?;
    for item in content {
        if let Some(text) = item.get("text").and_then(|value| value.as_str()) {
            out.push_str(text);
        }
    }
    Some(out)
}
