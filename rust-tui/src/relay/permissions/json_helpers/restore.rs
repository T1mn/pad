use super::remove::remove_json_path;
use super::set::{set_json_bool_path, set_json_string_path};

pub(in crate::relay::permissions) fn restore_json_string_path(
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

pub(in crate::relay::permissions) fn restore_json_bool_path(
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
