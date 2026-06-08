use crate::relay::common::{opencode_managed_state_path, read_json_value, write_json_value};
use serde_json::json;
use std::collections::BTreeSet;

pub(super) fn read_opencode_managed_keys() -> BTreeSet<String> {
    let state_path = opencode_managed_state_path();
    let value = read_json_value(&state_path, json!({ "provider_keys": [] }));
    value
        .get("provider_keys")
        .and_then(|items| items.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect()
}

pub(super) fn write_opencode_managed_keys(keys: &BTreeSet<String>) {
    let value = json!({
        "provider_keys": keys.iter().collect::<Vec<_>>()
    });
    write_json_value(&opencode_managed_state_path(), &value);
}
