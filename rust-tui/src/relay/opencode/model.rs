use std::collections::BTreeSet;

pub(super) fn sync_model_ref(
    root: &mut serde_json::Value,
    field: &str,
    selected_model: &str,
    valid_models: &BTreeSet<String>,
    previous_managed: &BTreeSet<String>,
) {
    if !selected_model.trim().is_empty() && valid_models.contains(selected_model) {
        root.as_object_mut().expect("opencode root object").insert(
            field.to_string(),
            serde_json::Value::String(selected_model.to_string()),
        );
    } else if model_ref_was_managed(root, field, previous_managed) {
        root.as_object_mut()
            .expect("opencode root object")
            .remove(field);
    }
}

fn model_ref_was_managed(
    root: &serde_json::Value,
    field: &str,
    previous_managed: &BTreeSet<String>,
) -> bool {
    root.get(field)
        .and_then(|value| value.as_str())
        .map(|value| {
            previous_managed
                .iter()
                .any(|key| value.starts_with(&format!("{key}/")))
        })
        .unwrap_or(false)
}
