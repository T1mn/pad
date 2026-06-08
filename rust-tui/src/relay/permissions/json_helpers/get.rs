pub(in crate::relay::permissions) fn json_string_at_path(
    value: &serde_json::Value,
    path: &[&str],
) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

pub(in crate::relay::permissions) fn json_bool_at_path(
    value: &serde_json::Value,
    path: &[&str],
) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}
