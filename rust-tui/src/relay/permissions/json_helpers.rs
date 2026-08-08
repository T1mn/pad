mod cleanup {
    pub(in crate::relay::permissions) fn cleanup_empty_json_objects(
        value: &mut serde_json::Value,
    ) -> bool {
        let Some(map) = value.as_object_mut() else {
            return false;
        };

        map.retain(|_, child| !cleanup_empty_json_objects(child));

        map.is_empty()
    }
}
mod get {
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
}
mod remove {
    pub(super) fn remove_json_path(value: &mut serde_json::Value, path: &[&str]) {
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
}
mod restore {
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
}
mod set;

pub(super) use cleanup::cleanup_empty_json_objects;
pub(super) use get::{json_bool_at_path, json_string_at_path};
pub(super) use restore::{restore_json_bool_path, restore_json_string_path};
pub(super) use set::{set_json_bool_path, set_json_string_path};
