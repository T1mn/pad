mod get {
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
}
mod remove {
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
}
mod restore {
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
}
mod set {
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
}

pub(super) use get::{toml_bool_at_path, toml_string_array_at_path};
pub(super) use remove::cleanup_empty_toml_table_path;
pub(super) use restore::{
    restore_toml_bool_path, restore_toml_string_array_path, restore_toml_string_field,
};
pub(super) use set::{set_toml_bool_path, set_toml_string_array_path};
