pub(super) fn restore_toml_string_field(
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

pub(super) fn set_toml_bool_path(
    root: &mut toml::map::Map<String, toml::Value>,
    path: &[&str],
    flag: bool,
) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let mut current = root;
    for key in parents {
        let entry = current
            .entry((*key).to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
        if !entry.is_table() {
            *entry = toml::Value::Table(toml::map::Map::new());
        }
        current = entry.as_table_mut().expect("nested toml table");
    }

    current.insert((*last).to_string(), toml::Value::Boolean(flag));
}

pub(super) fn set_toml_string_array_path(
    root: &mut toml::map::Map<String, toml::Value>,
    path: &[&str],
    values: &[&str],
) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let mut current = root;
    for key in parents {
        let entry = current
            .entry((*key).to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
        if !entry.is_table() {
            *entry = toml::Value::Table(toml::map::Map::new());
        }
        current = entry.as_table_mut().expect("nested toml table");
    }

    current.insert(
        (*last).to_string(),
        toml::Value::Array(
            values
                .iter()
                .map(|value| toml::Value::String((*value).to_string()))
                .collect(),
        ),
    );
}

pub(super) fn restore_toml_bool_path(
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

pub(super) fn restore_toml_string_array_path(
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

pub(super) fn toml_bool_at_path(
    root: &toml::map::Map<String, toml::Value>,
    path: &[&str],
) -> Option<bool> {
    let mut current = root.get(*path.first()?)?;
    for key in &path[1..] {
        current = current.as_table()?.get(*key)?;
    }
    current.as_bool()
}

pub(super) fn toml_string_array_at_path(
    root: &toml::map::Map<String, toml::Value>,
    path: &[&str],
) -> Option<Vec<String>> {
    let mut current = root.get(*path.first()?)?;
    for key in &path[1..] {
        current = current.as_table()?.get(*key)?;
    }
    current.as_array().map(|items| {
        items
            .iter()
            .filter_map(|value| value.as_str().map(ToString::to_string))
            .collect()
    })
}

pub(super) fn remove_toml_path(root: &mut toml::map::Map<String, toml::Value>, path: &[&str]) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let mut current = root;
    for key in parents {
        let Some(next) = current.get_mut(*key) else {
            return;
        };
        let Some(next_table) = next.as_table_mut() else {
            return;
        };
        current = next_table;
    }
    current.remove(*last);
}

pub(super) fn cleanup_empty_toml_table_path(
    root: &mut toml::map::Map<String, toml::Value>,
    path: &[&str],
) {
    if path.is_empty() {
        return;
    }

    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let mut current = root;
    for key in parents {
        let Some(next) = current.get_mut(*key) else {
            return;
        };
        let Some(next_table) = next.as_table_mut() else {
            return;
        };
        current = next_table;
    }

    let should_remove = current
        .get(*last)
        .and_then(|value| value.as_table())
        .map(|table| table.is_empty())
        .unwrap_or(false);
    if should_remove {
        current.remove(*last);
    }
}
