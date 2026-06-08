use super::remove::remove_toml_path;
use super::set::{set_toml_bool_path, set_toml_string_array_path};

pub(in crate::relay::permissions) fn restore_toml_string_field(
    root: &mut toml::map::Map<String, toml::Value>,
    key: &str,
    previous: Option<&serde_json::Value>,
) {
    if let Some(previous) = previous.and_then(|value| value.as_str()) {
        root.insert(key.to_string(), toml::Value::String(previous.to_string()));
    } else {
        root.remove(key);
    }
}

pub(in crate::relay::permissions) fn restore_toml_bool_path(
    root: &mut toml::map::Map<String, toml::Value>,
    path: &[&str],
    previous: Option<&serde_json::Value>,
) {
    if let Some(previous) = previous.and_then(|value| value.as_bool()) {
        set_toml_bool_path(root, path, previous);
    } else {
        remove_toml_path(root, path);
    }
}

pub(in crate::relay::permissions) fn restore_toml_string_array_path(
    root: &mut toml::map::Map<String, toml::Value>,
    path: &[&str],
    previous: Option<&serde_json::Value>,
) {
    if let Some(previous) = previous.and_then(|value| value.as_array()) {
        let values: Vec<&str> = previous.iter().filter_map(|value| value.as_str()).collect();
        set_toml_string_array_path(root, path, &values);
    } else {
        remove_toml_path(root, path);
    }
}
