pub(in crate::relay::permissions) fn toml_bool_at_path(
    root: &toml::map::Map<String, toml::Value>,
    path: &[&str],
) -> Option<bool> {
    toml_value_at_path(root, path)?.as_bool()
}

pub(in crate::relay::permissions) fn toml_string_array_at_path(
    root: &toml::map::Map<String, toml::Value>,
    path: &[&str],
) -> Option<Vec<String>> {
    toml_value_at_path(root, path)?.as_array().map(|items| {
        items
            .iter()
            .filter_map(|value| value.as_str().map(ToString::to_string))
            .collect()
    })
}

fn toml_value_at_path<'a>(
    root: &'a toml::map::Map<String, toml::Value>,
    path: &[&str],
) -> Option<&'a toml::Value> {
    let mut current = root.get(*path.first()?)?;
    for key in &path[1..] {
        current = current.as_table()?.get(*key)?;
    }
    Some(current)
}
