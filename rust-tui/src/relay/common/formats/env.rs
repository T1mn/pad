use std::collections::BTreeMap;

pub(in crate::relay) fn parse_env_file(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

pub(in crate::relay) fn serialize_env_file(map: &BTreeMap<String, String>) -> String {
    if map.is_empty() {
        return String::new();
    }

    let mut serialized = map
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    serialized.push('\n');
    serialized
}
