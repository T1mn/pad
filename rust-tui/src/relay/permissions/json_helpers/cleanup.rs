pub(in crate::relay::permissions) fn cleanup_empty_json_objects(
    value: &mut serde_json::Value,
) -> bool {
    let Some(map) = value.as_object_mut() else {
        return false;
    };

    let keys = map.keys().cloned().collect::<Vec<_>>();
    for key in keys {
        let remove_key = map
            .get_mut(&key)
            .map(cleanup_empty_json_objects)
            .unwrap_or(false);
        if remove_key {
            map.remove(&key);
        }
    }

    map.is_empty()
}
