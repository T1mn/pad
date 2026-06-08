use crate::relay::common::{parse_toml_document, serialize_toml_document};

pub(super) fn update_codex_provider_config(
    content: &str,
    provider_name: &str,
    provider_label: &str,
    base_url: &str,
) -> String {
    let mut doc = parse_toml_document(content);

    let root = doc.as_table_mut().expect("root toml value must be a table");
    upsert_codex_provider_config(root, provider_name, provider_label, base_url);

    serialize_toml_document(&doc)
}

fn upsert_codex_provider_config(
    root: &mut toml::map::Map<String, toml::Value>,
    provider_name: &str,
    provider_label: &str,
    base_url: &str,
) {
    root.insert(
        "model_provider".to_string(),
        toml::Value::String(provider_name.to_string()),
    );

    let providers = root
        .entry("model_providers")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

    if !providers.is_table() {
        *providers = toml::Value::Table(toml::map::Map::new());
    }

    let providers_table = providers
        .as_table_mut()
        .expect("model_providers must be a table");
    let provider_entry = providers_table
        .entry(provider_name.to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

    if !provider_entry.is_table() {
        *provider_entry = toml::Value::Table(toml::map::Map::new());
    }

    let provider_table = provider_entry
        .as_table_mut()
        .expect("provider entry must be a table");
    provider_table.insert(
        "base_url".to_string(),
        toml::Value::String(base_url.to_string()),
    );
    provider_table.insert(
        "name".to_string(),
        toml::Value::String(provider_label.to_string()),
    );
    provider_table.insert(
        "requires_openai_auth".to_string(),
        toml::Value::Boolean(true),
    );
    provider_table.remove("env_key");
}

pub(super) fn current_model_provider(content: &str) -> Option<String> {
    let doc = parse_toml_document(content);
    doc.get("model_provider")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}
