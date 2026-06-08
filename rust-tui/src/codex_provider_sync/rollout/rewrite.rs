use serde_json::Value;
use std::io;

pub(super) fn rewrite_rollout_first_line(
    first_line: &str,
    target_provider: &str,
) -> io::Result<Option<String>> {
    if first_line.trim().is_empty() {
        return Ok(None);
    }

    let mut value = match serde_json::from_str::<Value>(first_line) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    if !is_session_meta(&value) {
        return Ok(None);
    }

    let Some(payload) = value.get_mut("payload").and_then(Value::as_object_mut) else {
        return Ok(None);
    };

    let current_provider = payload
        .get("model_provider")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if current_provider == target_provider {
        return Ok(None);
    }

    payload.insert(
        "model_provider".to_string(),
        Value::String(target_provider.to_string()),
    );
    serde_json::to_string(&value)
        .map(Some)
        .map_err(|err| io::Error::other(err.to_string()))
}

fn is_session_meta(value: &Value) -> bool {
    value
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|value| value == "session_meta")
}
