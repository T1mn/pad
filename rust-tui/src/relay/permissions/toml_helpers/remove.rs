pub(super) fn remove_toml_path(root: &mut toml::map::Map<String, toml::Value>, path: &[&str]) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let Some(current) = toml_parent_table_mut(root, parents) else {
        return;
    };
    current.remove(*last);
}

pub(in crate::relay::permissions) fn cleanup_empty_toml_table_path(
    root: &mut toml::map::Map<String, toml::Value>,
    path: &[&str],
) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };

    let Some(current) = toml_parent_table_mut(root, parents) else {
        return;
    };
    let should_remove = current
        .get(*last)
        .and_then(|value| value.as_table())
        .map(|table| table.is_empty())
        .unwrap_or(false);
    if should_remove {
        current.remove(*last);
    }
}

fn toml_parent_table_mut<'a>(
    root: &'a mut toml::map::Map<String, toml::Value>,
    parents: &[&str],
) -> Option<&'a mut toml::map::Map<String, toml::Value>> {
    let mut current = root;
    for key in parents {
        let next = current.get_mut(*key)?;
        current = next.as_table_mut()?;
    }
    Some(current)
}
