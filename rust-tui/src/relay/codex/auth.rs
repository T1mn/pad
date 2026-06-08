use serde_json::json;

pub(super) fn update_codex_auth_config(content: &str, api_key: &str) -> String {
    let mut obj = serde_json::from_str::<serde_json::Value>(content).unwrap_or_else(|_| json!({}));
    if !obj.is_object() {
        obj = json!({});
    }
    obj["auth_mode"] = serde_json::Value::String("apikey".to_string());
    obj["OPENAI_API_KEY"] = serde_json::Value::String(api_key.to_string());
    let mut serialized = serde_json::to_string_pretty(&obj).unwrap_or_default();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    serialized
}
