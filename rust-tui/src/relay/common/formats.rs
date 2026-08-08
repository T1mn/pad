mod env {
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

        let mut serialized = String::new();
        for (key, value) in map {
            serialized.push_str(key);
            serialized.push('=');
            serialized.push_str(value);
            serialized.push('\n');
        }
        serialized
    }
}
mod json;
mod toml {
    pub(in crate::relay) fn parse_toml_document(content: &str) -> toml::Value {
        content
            .parse::<toml::Value>()
            .unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
    }

    pub(in crate::relay) fn serialize_toml_document(value: &toml::Value) -> String {
        let mut serialized = toml::to_string(value).unwrap_or_default();
        if !serialized.ends_with('\n') {
            serialized.push('\n');
        }
        serialized
    }
}

pub(in crate::relay) use env::{parse_env_file, serialize_env_file};
pub(in crate::relay) use json::{
    parse_json_object, parse_json_object_strict, read_json_object_for_update, read_json_value,
    serialize_json_pretty, write_json_value,
};
pub(in crate::relay) use toml::{parse_toml_document, serialize_toml_document};
