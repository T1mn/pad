use serde_json::Value;

pub(super) fn parse_subagent_parent_thread_id(source: Option<&str>) -> Option<String> {
    let source = source?.trim();
    if source.is_empty() || !source.starts_with('{') {
        return None;
    }

    let value = serde_json::from_str::<Value>(source).ok()?;
    value
        .get("subagent")
        .and_then(|subagent| subagent.get("thread_spawn"))
        .and_then(|spawn| spawn.get("parent_thread_id"))
        .and_then(|parent| parent.as_str())
        .map(|parent| parent.to_string())
}
