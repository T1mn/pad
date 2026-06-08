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
