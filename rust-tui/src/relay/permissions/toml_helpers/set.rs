pub(in crate::relay::permissions) fn set_toml_bool_path(
    root: &mut toml::map::Map<String, toml::Value>,
    path: &[&str],
    flag: bool,
) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let current = ensure_toml_parent_table(root, parents);
    current.insert((*last).to_string(), toml::Value::Boolean(flag));
}

pub(in crate::relay::permissions) fn set_toml_string_array_path(
    root: &mut toml::map::Map<String, toml::Value>,
    path: &[&str],
    values: &[&str],
) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let current = ensure_toml_parent_table(root, parents);
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

fn ensure_toml_parent_table<'a>(
    root: &'a mut toml::map::Map<String, toml::Value>,
    parents: &[&str],
) -> &'a mut toml::map::Map<String, toml::Value> {
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
    current
}
