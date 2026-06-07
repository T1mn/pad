use serde_json::json;

pub(super) fn json_string_at_path(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

pub(super) fn json_bool_at_path(value: &serde_json::Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}

pub(super) fn set_json_string_path(value: &mut serde_json::Value, path: &[&str], text: &str) {
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

pub(super) fn set_json_bool_path(value: &mut serde_json::Value, path: &[&str], flag: bool) {
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

pub(super) fn restore_json_string_path(
    value: &mut serde_json::Value,
    path: &[&str],
    previous: Option<&serde_json::Value>,
) {
    if let Some(previous) = previous.and_then(|value| value.as_str()) {
        set_json_string_path(value, path, previous);
    } else {
        remove_json_path(value, path);
    }
}

pub(super) fn restore_json_bool_path(
    value: &mut serde_json::Value,
    path: &[&str],
    previous: Option<&serde_json::Value>,
) {
    if let Some(previous) = previous.and_then(|value| value.as_bool()) {
        set_json_bool_path(value, path, previous);
    } else {
        remove_json_path(value, path);
    }
}

fn remove_json_path(value: &mut serde_json::Value, path: &[&str]) {
    if path.is_empty() {
        return;
    }
    let Some(root) = value.as_object_mut() else {
        return;
    };
    remove_json_path_in_map(root, path);
}

fn remove_json_path_in_map(
    map: &mut serde_json::Map<String, serde_json::Value>,
    path: &[&str],
) -> bool {
    if path.len() == 1 {
        map.remove(path[0]);
        return map.is_empty();
    }

    let remove_child = if let Some(child) = map.get_mut(path[0]) {
        if let Some(child_map) = child.as_object_mut() {
            remove_json_path_in_map(child_map, &path[1..])
        } else {
            false
        }
    } else {
        false
    };

    if remove_child {
        map.remove(path[0]);
    }

    map.is_empty()
}

pub(super) fn cleanup_empty_json_objects(value: &mut serde_json::Value) -> bool {
    let Some(map) = value.as_object_mut() else {
        return false;
    };

    let keys = map.keys().cloned().collect::<Vec<_>>();
    for key in keys {
        let remove_key = map
            .get_mut(&key)
            .map(cleanup_empty_json_objects)
            .unwrap_or(false);
        if remove_key {
            map.remove(&key);
        }
    }

    map.is_empty()
}
