use serde_json::Value;

pub(crate) fn is_subagent_source(source: Option<&str>) -> bool {
    let Some(source) = source else {
        return false;
    };
    let source = source.trim();
    if source.is_empty() || !source.starts_with('{') {
        return false;
    }

    let Ok(value) = serde_json::from_str::<Value>(source) else {
        return false;
    };
    value.get("subagent").is_some_and(|value| !value.is_null())
}
