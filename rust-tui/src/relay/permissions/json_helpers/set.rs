use serde_json::json;

pub(in crate::relay::permissions) fn set_json_string_path(
    value: &mut serde_json::Value,
    path: &[&str],
    text: &str,
) {
    let mut current = value;
    for key in &path[..path.len().saturating_sub(1)] {
        current = ensure_json_child_value(current, key);
    }
    if let Some(last) = path.last() {
        current.as_object_mut().expect("json object").insert(
            (*last).to_string(),
            serde_json::Value::String(text.to_string()),
        );
    }
}

pub(in crate::relay::permissions) fn set_json_bool_path(
    value: &mut serde_json::Value,
    path: &[&str],
    flag: bool,
) {
    let mut current = value;
    for key in &path[..path.len().saturating_sub(1)] {
        current = ensure_json_child_value(current, key);
    }
    if let Some(last) = path.last() {
        current
            .as_object_mut()
            .expect("json object")
            .insert((*last).to_string(), serde_json::Value::Bool(flag));
    }
}

fn ensure_json_child_value<'a>(
    value: &'a mut serde_json::Value,
    key: &str,
) -> &'a mut serde_json::Value {
    let object = value.as_object_mut().expect("json object");
    let entry = object.entry(key.to_string()).or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    entry
}
