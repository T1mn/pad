use serde_json::Value;

pub(super) fn parse_model(raw: &Option<String>) -> (Option<String>, Option<String>) {
    let Some(raw) = raw.as_deref() else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return (None, None);
    };
    let provider = value
        .get("providerID")
        .or_else(|| value.get("provider"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let model = value
        .get("modelID")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    (provider, model)
}
