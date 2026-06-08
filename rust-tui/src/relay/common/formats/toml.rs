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
