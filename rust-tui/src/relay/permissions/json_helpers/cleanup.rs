pub(in crate::relay::permissions) fn cleanup_empty_json_objects(
    value: &mut serde_json::Value,
) -> bool {
    let Some(map) = value.as_object_mut() else {
        return false;
    };

    map.retain(|_, child| !cleanup_empty_json_objects(child));

    map.is_empty()
}
